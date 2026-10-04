#!/usr/bin/env python3
"""extract: pick a destination for an archive from a fuzzel menu, then
extract it with a zenity progress bar.

Usage: extract <archive>

Everything is extracted into a hidden temporary folder next to the
destination first, then moved into place, so nothing is ever overwritten
(an existing name gets -2, -3, ...) and a cancelled or failed extraction
leaves nothing behind.

"Smart" destination follows file-roller's extract-here rule: an archive
with a single top-level entry is extracted as-is, one with several
top-level entries is wrapped in a folder named after the archive. So no
foo/foo/ and no archive spilling files everywhere.

Backends (all with a progress bar):
  zip family   Python zipfile   exact, restores Unix permissions + symlinks
  tar (+gz/bz2/xz/lzma)  Python tarfile  exact, "data" safety filter
  7z, rar, encrypted zip   7-Zip (7zz)   its own overall percentage
  single .gz/.bz2/.xz/.zst file   exact, by compressed bytes read
  anything else (tar.zst, iso, cpio...)   bsdtar, counts files
"""

import bz2
import gzip
import lzma
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

ZIP_EXT = (".zip", ".jar", ".apk", ".epub", ".whl", ".xpi", ".cbz", ".aar")
TAR_EXT = (".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tbz",
           ".tar.xz", ".txz", ".tar.lzma", ".tlz")
# 7-Zip opens all of these (checked against 7-Zip 26.02's format list)
SEVENZIP_EXT = (".7z", ".rar", ".cbr", ".cb7", ".zipx", ".001",
                ".iso", ".arj", ".cab", ".chm", ".msi", ".dmg", ".wim",
                ".esd", ".swm", ".xar", ".pkg", ".vhd", ".vhdx", ".vmdk",
                ".vdi", ".qcow2", ".squashfs", ".lzh", ".lha")
# bsdtar (libarchive); tar.lz4 uses the lz4 program, tar.lz uses liblzma
TAR_OTHER_EXT = (".tar.zst", ".tzst", ".tar.lz4", ".tar.lz", ".tar.z",
                 ".taz", ".cpio", ".deb", ".rpm")
SINGLE_EXT = {".gz": "gzip", ".bz2": "bz2", ".xz": "xz", ".lzma": "xz",
              ".zst": ["zstd", "-dcq"], ".lz4": ["lz4", "-dcq"],
              ".lz": ["lzip", "-dcq"],
              ".z": ["gzip", "-dcq"]}  # gzip also reads old compress (.Z)
STRIP_EXT = TAR_EXT + TAR_OTHER_EXT + ZIP_EXT + SEVENZIP_EXT


class Cancelled(Exception):
    pass


# ── small helpers ────────────────────────────────────────────────────────

def notify(body, urgent=False):
    cmd = ["notify-send", "-a", "extract"]
    if urgent:
        cmd += ["-u", "critical"]
    subprocess.run(cmd + ["extract", body], check=False)


def fuzzel(options, prompt, lines=None, width=None):
    """Show a fuzzel menu; return the chosen line or None."""
    cmd = ["fuzzel", "--dmenu", "--prompt", prompt]
    cmd += ["--lines", str(lines or min(len(options), 10))]
    if width:
        cmd += ["--width", str(width)]
    res = subprocess.run(cmd, input="\n".join(options) + "\n",
                         capture_output=True, text=True, check=False)
    choice = res.stdout.strip("\n")
    return choice if res.returncode == 0 and choice else None


VOLUME_RE = (
    re.compile(r"^(.*)\.part(\d+)\.rar$", re.I),   # x.part1.rar, x.part01.rar
    re.compile(r"^(.*\.[^.]+)\.(\d{3})$", re.I),     # x.7z.001, x.zip.001
)


def stem_of(path):
    for rx in VOLUME_RE:
        m = rx.match(path.name)
        if m:
            return stem_of(Path(m.group(1))) if rx is VOLUME_RE[1] else m.group(1)
    name = path.name
    low = name.lower()
    for ext in sorted(STRIP_EXT, key=len, reverse=True):
        if low.endswith(ext) and len(name) > len(ext):
            return name[: -len(ext)]
    return path.stem or name


def unique(path, is_dir=True):
    """path, or name-2, name-3... whichever doesn't exist yet.
    Files keep their extension: notes.txt -> notes-2.txt."""
    path = Path(path)
    if not os.path.lexists(path):
        return path
    base, ext = path.name, ""
    if not is_dir and path.suffix and path.stem:
        base, ext = path.stem, path.suffix
    n = 2
    while os.path.lexists(path.with_name(f"{base}-{n}{ext}")):
        n += 1
    return path.with_name(f"{base}-{n}{ext}")


def top_level(names):
    """Distinct top-level entries of an archive listing."""
    tops = set()
    for name in names:
        parts = [p for p in PurePosixPath(name.replace("\\", "/")).parts
                 if p not in ("", ".", "/")]
        if parts:
            tops.add(parts[0])
    return sorted(tops)


def inside(root, target):
    root = os.path.realpath(root)
    target = os.path.realpath(target)
    return target == root or target.startswith(root + os.sep)


def sevenzip():
    return shutil.which("7zz") or shutil.which("7z")


# ── progress dialog ──────────────────────────────────────────────────────

class Progress:
    """zenity --progress driven over stdin. A closed dialog = cancel."""

    def __init__(self, title, text, pulsate=False):
        cmd = ["zenity", "--progress", "--title", title, "--text", text,
               "--auto-close", "--width", "420"]
        if pulsate:
            cmd.append("--pulsate")
        self.proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, text=True)
        self.last = -1

    def cancelled(self):
        return self.proc.poll() is not None

    def update(self, pct):
        pct = max(0, min(99, int(pct)))  # 100 is sent by close()
        if self.cancelled():
            raise Cancelled
        if pct != self.last:
            self.last = pct
            try:
                self.proc.stdin.write(f"{pct}\n")
                self.proc.stdin.flush()
            except (BrokenPipeError, ValueError) as err:
                raise Cancelled from err

    def close(self, done):
        try:
            if done:
                self.proc.stdin.write("100\n")
            self.proc.stdin.close()
        except (BrokenPipeError, ValueError):
            pass
        if not done:
            self.proc.terminate()
        self.proc.wait()


# ── backends ─────────────────────────────────────────────────────────────

def junk(name):
    """macOS metadata that zips from Finder carry along."""
    return name.startswith("__MACOSX/") or PurePosixPath(name).name == ".DS_Store"


class ZipBackend:
    pulsate = False

    def __init__(self, path):
        self.path = path

    def names(self):
        with zipfile.ZipFile(self.path) as zf:
            return [n for n in zf.namelist() if not junk(n)]

    def extract(self, dest, progress):
        with zipfile.ZipFile(self.path) as zf:
            infos = [i for i in zf.infolist() if not junk(i.filename)]
            dir_modes = []
            for i, info in enumerate(infos, 1):
                progress.update(100 * (i - 1) / max(len(infos), 1))
                mode = info.external_attr >> 16 if info.create_system == 3 else 0
                if stat.S_ISLNK(mode):
                    self._symlink(zf, info, dest)
                    continue
                out = zf.extract(info, dest)  # zipfile strips / and ..
                if mode & 0o777:
                    if info.is_dir():
                        dir_modes.append((out, mode & 0o777))
                    else:
                        os.chmod(out, (mode & 0o777) | 0o600)
            # directories last, so read-only ones don't block their contents
            for out, mode in reversed(dir_modes):
                os.chmod(out, mode | 0o700)

    @staticmethod
    def _symlink(zf, info, dest):
        """Recreate a symlink, but only if it stays inside dest."""
        target = zf.read(info).decode("utf-8", "replace")
        rel = PurePosixPath(info.filename)
        parts = [p for p in rel.parts if p not in ("", ".", "..", "/")]
        if not parts or target.startswith("/"):
            return
        link = Path(dest, *parts)
        link.parent.mkdir(parents=True, exist_ok=True)
        if not inside(dest, link.parent / target):
            return
        if not os.path.lexists(link):
            os.symlink(target, link)


class TarBackend:
    pulsate = False

    def __init__(self, path):
        self.path = path

    def names(self):
        with tarfile.open(self.path) as tf:
            return tf.getnames()

    def extract(self, dest, progress):
        with tarfile.open(self.path) as tf:
            total = max(len(tf.getmembers()), 1)
            count = [0]

            def data_with_progress(member, path):
                count[0] += 1
                progress.update(100 * (count[0] - 1) / total)
                try:
                    return tarfile.data_filter(member, path)
                except tarfile.FilterError:
                    return None  # unsafe entry (absolute, ../, device): skip it

            tf.extractall(dest, filter=data_with_progress)


class SevenZipBackend:
    """7-Zip: 7z, rar, encrypted zips. Reports whole-archive %."""
    pulsate = False
    PCT = re.compile(rb"(\d{1,3})%")

    def __init__(self, path):
        self.path = path
        self.password = None

    def _list(self, password=None):
        cmd = [sevenzip(), "l", "-slt", "-p" + (password or ""), str(self.path)]
        return subprocess.run(cmd, capture_output=True, stdin=subprocess.DEVNULL,
                              check=False)

    def names(self):
        res = self._list()
        # encrypted file names: nothing can be listed without the password
        for _ in range(3):
            text = (res.stdout + res.stderr).decode("utf-8", "replace")
            if res.returncode == 0 or "encrypted archive" not in text.lower():
                break
            self.password = ask_password(self.path, retry=self.password is not None)
            res = self._list(self.password)
        if res.returncode != 0:
            text = (res.stdout + res.stderr).decode("utf-8", "replace")
            if "encrypted archive" in text.lower():
                raise RuntimeError("wrong password")
            raise RuntimeError("7-Zip couldn't read the archive")
        body = res.stdout.decode("utf-8", "replace").split("\n----------\n", 1)[-1]
        names = re.findall(r"^Path = (.*)$", body, re.M)
        # encrypted contents only: names list fine, extraction needs it
        if "Encrypted = +" in body and self.password is None:
            self.password = ask_password(self.path)
        return names

    def extract(self, dest, progress):
        cmd = [sevenzip(), "x", "-y", "-bsp1", "-bb0", "-bso0",
               "-p" + (self.password or ""), f"-o{dest}", str(self.path)]
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, stdin=subprocess.DEVNULL)
        try:
            buf = b""
            while True:
                chunk = proc.stdout.read(256)
                if not chunk:
                    break
                buf = (buf + chunk)[-64:]
                found = self.PCT.findall(buf)
                if found:
                    progress.update(int(found[-1]))
                elif progress.cancelled():
                    raise Cancelled
        except Cancelled:
            proc.kill()
            proc.wait()
            raise
        err = proc.stderr.read().decode("utf-8", "replace").strip()
        if proc.wait() != 0:
            if "wrong password" in err.lower():
                raise RuntimeError("wrong password")
            raise RuntimeError(err.splitlines()[-1] if err else "7-Zip failed")


class SingleFileBackend:
    """A lone compressed file (x.gz, x.zst...). Progress by bytes read."""
    pulsate = False
    OPENERS = {"gzip": gzip.open, "bz2": bz2.open, "xz": lzma.open}

    def __init__(self, path, kind, stem):
        self.path, self.kind, self.stem = path, kind, stem

    def names(self):
        return [self.stem]

    def extract(self, dest, progress):
        size = max(self.path.stat().st_size, 1)
        out_path = Path(dest, self.stem)
        with open(self.path, "rb") as raw, open(out_path, "wb") as out:
            if isinstance(self.kind, list):  # an external decompressor
                proc = subprocess.Popen(self.kind, stdin=subprocess.PIPE,
                                        stdout=out)
                try:
                    while chunk := raw.read(1 << 20):
                        proc.stdin.write(chunk)
                        progress.update(100 * raw.tell() / size)
                except Cancelled:
                    proc.kill()
                    proc.wait()
                    raise
                proc.stdin.close()
                if proc.wait() != 0:
                    raise RuntimeError(f"{self.kind[0]} failed")
                return
            with self.OPENERS[self.kind](raw) as src:
                while chunk := src.read(1 << 20):
                    out.write(chunk)
                    progress.update(100 * raw.tell() / size)


class BsdtarBackend:
    """Everything libarchive reads. Progress by counting files."""
    pulsate = False

    def __init__(self, path):
        self.path = path
        self.total = 0

    def names(self):
        res = subprocess.run(["bsdtar", "-tf", str(self.path)],
                             capture_output=True, text=True, check=False)
        if res.returncode != 0:
            raise RuntimeError(res.stderr.strip() or "unsupported archive")
        names = res.stdout.splitlines()
        self.total = len(names)
        return names

    def extract(self, dest, progress):
        # bsdtar refuses absolute and ../ paths by default
        proc = subprocess.Popen(["bsdtar", "-xvf", str(self.path), "-C", str(dest)],
                                stderr=subprocess.PIPE, text=True)
        done = 0
        try:
            for line in proc.stderr:
                if line.startswith("x "):
                    done += 1
                    progress.update(100 * done / max(self.total, 1))
        except Cancelled:
            proc.kill()
            proc.wait()
            raise
        if proc.wait() != 0:
            raise RuntimeError("bsdtar failed")


def first_volume(path):
    """For any part of a split archive, the part to open (part 1).
    Returns (path, is_split). Raises if part 1 is missing."""
    name, parent = path.name, path.parent
    checks = []
    m = re.match(r"^(.*)\.part(\d+)\.rar$", name, re.I)
    if m:
        checks.append(f"{m.group(1)}.part{'1'.zfill(len(m.group(2)))}.rar")
    m = re.match(r"^(.*)\.r\d\d$", name, re.I)              # old style: x.r00
    if m:
        checks.append(f"{m.group(1)}.rar")
    m = re.match(r"^(.*)\.(\d{3})$", name)                  # x.7z.002
    if m:
        checks.append(f"{m.group(1)}.001")
    m = re.match(r"^(.*)\.z\d\d$", name, re.I)              # spanned zip: x.z01
    if m:
        checks.append(f"{m.group(1)}.zip")
    if not checks:
        spanned = path.suffix.lower() == ".zip" and (
            Path(parent, path.stem + ".z01").exists()
            or Path(parent, path.stem + ".Z01").exists())
        return path, spanned
    first = Path(parent, checks[0])
    if not first.exists():
        raise RuntimeError(f"first part not found: {first.name}")
    return first, True


def backend_for(path, split=False):
    if split:
        return SevenZipBackend(path)
    low = path.name.lower()
    if low.endswith(TAR_EXT) and tarfile.is_tarfile(path):
        return TarBackend(path)
    if low.endswith(TAR_OTHER_EXT):  # before 7-Zip: .tar.z also ends in .z
        return BsdtarBackend(path)
    if low.endswith(SEVENZIP_EXT):
        return SevenZipBackend(path)
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as zf:
            if any(i.flag_bits & 0x1 for i in zf.infolist()):
                return SevenZipBackend(path)  # encrypted zip
        return ZipBackend(path)
    for ext, kind in SINGLE_EXT.items():
        if low.endswith(ext):
            return SingleFileBackend(path, kind, path.name[: -len(ext)])
    if tarfile.is_tarfile(path):
        return TarBackend(path)
    return BsdtarBackend(path)


def ask_password(path, retry=False):
    title = f"{'wrong password, try again' if retry else 'password'}: {path.name}"
    res = subprocess.run(["zenity", "--password", "--title", title],
                         capture_output=True, text=True, check=False)
    if res.returncode != 0:
        raise Cancelled
    return res.stdout.rstrip("\n")


# ── placing the result ───────────────────────────────────────────────────

def place(tmp, parent, wrap_name):
    """Move tmp's contents into parent, never overwriting.

    wrap_name set:  tmp itself becomes parent/<wrap_name>(-2...)
    wrap_name None: each top-level item moves into parent (renamed if taken)
    Returns the path to show the user.
    """
    if wrap_name:
        target = unique(Path(parent, wrap_name))
        os.rename(tmp, target)
        return target
    items = sorted(os.listdir(tmp))
    shown = Path(parent)
    for item in items:
        target = unique(Path(parent, item), is_dir=os.path.isdir(Path(tmp, item)))
        os.rename(Path(tmp, item), target)
        if len(items) == 1:
            shown = target
    os.rmdir(tmp)
    return shown


def run(archive):
    archive = Path(archive).resolve()
    if not archive.is_file():
        notify(f"not a file: {archive}", urgent=True)
        return 1

    clicked = archive
    archive, split = first_volume(archive)
    if archive != clicked:
        notify(f"using the first part: {archive.name}")
    name = stem_of(archive)
    backend = backend_for(archive, split)
    names = backend.names()
    tops = top_level(names)
    wrapped = len(tops) == 1

    if wrapped:
        smart = f"extract here  →  {tops[0]}"
    else:
        smart = f"extract to  {name}/  ({len(tops)} items)"
    loose = f"extract here, no folder  ({len(tops)} item{'s' if len(tops) != 1 else ''})"
    options = [smart]
    if not wrapped:
        options.append(loose)
    options += [f"extract to  /tmp/{name}", "choose folder…", "list contents"]

    choice = fuzzel(options, f"{archive.name} › ", width=60)
    if choice is None:
        return 0
    if choice == "list contents":
        fuzzel(names or ["(empty)"], "contents › ", lines=20, width=90)
        return 0

    if choice == smart:
        parent, wrap = archive.parent, (None if wrapped else name)
    elif choice == loose:
        parent, wrap = archive.parent, None
    elif choice.startswith("extract to  /tmp/"):
        parent, wrap = Path(tempfile.gettempdir()), (None if wrapped else name)
    else:
        res = subprocess.run(["zenity", "--file-selection", "--directory",
                              "--title", f"extract {archive.name} to…",
                              f"--filename={archive.parent}/"],
                             capture_output=True, text=True, check=False)
        if res.returncode != 0 or not res.stdout.strip():
            return 0
        parent, wrap = Path(res.stdout.strip()), (None if wrapped else name)

    tmp = Path(tempfile.mkdtemp(prefix=f".extract-{name}-", dir=parent))
    progress = Progress("extract", f"extracting {archive.name}…", backend.pulsate)
    try:
        backend.extract(tmp, progress)
        progress.close(done=True)
        result = place(tmp, parent, wrap)
    except Cancelled:
        progress.close(done=False)
        shutil.rmtree(tmp, ignore_errors=True)
        notify(f"cancelled: {archive.name}")
        return 1
    except Exception as err:  # noqa: BLE001  (show any failure to the user)
        progress.close(done=False)
        shutil.rmtree(tmp, ignore_errors=True)
        notify(f"failed: {archive.name}\n{err}", urgent=True)
        return 1

    notify(f"done → {result}")
    after = fuzzel(["open in thunar", "open terminal here", "copy path", "done"],
                   "› ", lines=4)
    folder = result if result.is_dir() else result.parent
    if after == "open in thunar":
        subprocess.Popen(["thunar", str(folder)], start_new_session=True,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    elif after == "open terminal here":
        subprocess.Popen(["kitty", "--directory", str(folder)], start_new_session=True,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    elif after == "copy path":
        subprocess.run(["wl-copy", str(result)], check=False)
    return 0


def main():
    if len(sys.argv) != 2:
        print("usage: extract <archive>", file=sys.stderr)
        return 2
    try:
        return run(sys.argv[1])
    except Cancelled:
        return 1
    except Exception as err:  # noqa: BLE001
        notify(f"failed: {Path(sys.argv[1]).name}\n{err}", urgent=True)
        return 1


if __name__ == "__main__":
    sys.exit(main())
