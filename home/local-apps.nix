{ pkgs, lib, ... }:
let
  glibcPath = "${pkgs.glibc}/lib/ld-linux-x86-64.so.2";
  rpath = lib.makeLibraryPath [
    pkgs.openssl
    pkgs.wayland
    pkgs.libxkbcommon
    pkgs.libGL
    pkgs.gcc.cc.lib
  ];
in
{
  home.activation.localApps = lib.hm.dag.entryAfter ["writeBoundary"] ''
    mkdir -p "$HOME/.bin"

    # Download a release binary if it's missing. A failed download (no
    # network yet, or NetworkManager restarting during a rebuild) only
    # warns, so it can't fail the rebuild; the next rebuild tries again.
    # Downloads go to a temp file first, so an error page or a partial
    # file never ends up in ~/.bin.
    la_fetch() {
      local dest="$HOME/.bin/$1" url=$2
      [ -f "$dest" ] && return 0
      if ${pkgs.curl}/bin/curl -fsSL --retry 3 --retry-connrefused \
           --connect-timeout 10 "$url" -o "$dest.part"; then
        mv "$dest.part" "$dest"
      else
        rm -f "$dest.part"
        echo "localApps: couldn't download $1, will retry on the next rebuild" >&2
        return 1
      fi
    }
    # patchelf fails on static binaries (no .interp), that's fine
    la_fix() {
      local f="$HOME/.bin/$1"; shift
      [ -f "$f" ] || return 0
      ${pkgs.patchelf}/bin/patchelf --set-interpreter ${glibcPath} "$@" "$f" || true
      chmod +x "$f"
    }

    la_fetch uwuplsplay "https://github.com/ilovealienz/uwuplsplay/releases/latest/download/uwuplsplay-linux" || true
    la_fetch stremio-cliuwu "https://github.com/ilovealienz/stremio-cliuwu/releases/latest/download/stremio-cliuwu-linux-amd64" || true
    la_fetch zipline-upload "https://github.com/ilovealienz/my-zipline-uploader/releases/latest/download/zipline-upload" || true
    la_fix uwuplsplay
    la_fix stremio-cliuwu
    la_fix zipline-upload --set-rpath ${rpath}

    # uwuplsplay mime/protocol registration
    mkdir -p "$HOME/.local/share/applications" "$HOME/.local/share/mime/packages"
    cat > "$HOME/.local/share/applications/uwuplsplay.desktop" << EOF
[Desktop Entry]
Name=uwuplsplay
Exec=$HOME/.bin/uwuplsplay %u
MimeType=application/x-uwuplsplay;x-scheme-handler/uwupls;
Type=Application
NoDisplay=true
EOF
    cat > "$HOME/.local/share/mime/packages/uwuplsplay.xml" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-uwuplsplay">
    <comment>uwuplsplay stream link</comment>
    <glob pattern="*.uwuplsplay"/>
  </mime-type>
</mime-info>
EOF
    ${pkgs.shared-mime-info}/bin/update-mime-database "$HOME/.local/share/mime" >/dev/null 2>&1 || true
    ${pkgs.xdg-utils}/bin/xdg-mime default uwuplsplay.desktop application/x-uwuplsplay || true
    ${pkgs.xdg-utils}/bin/xdg-mime default uwuplsplay.desktop x-scheme-handler/uwupls || true
    ${pkgs.desktop-file-utils}/bin/update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
  '';
}
