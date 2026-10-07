"""Make sure our Thunar custom actions exist in ~/.config/Thunar/uca.xml,
without taking the file away from Thunar.

Run by home-manager on every rebuild. The file stays a normal file, so
Thunar's "Configure custom actions" window keeps working and your own
actions and edits are kept. For each action we manage (matched by its
unique-id):
  - missing       -> it's added
  - present       -> left as you have it, except:
                     <command>  updated while it still points at our program
                                (a new Nix store path after a rebuild) or at
                                the old default we replace
                     <patterns> updated while they still look like ours
                                (so new file types show up), never if edited
If uca.xml isn't valid XML it's left untouched with a warning, so a broken
file can never make a rebuild fail.
Usage: uca_merge.py <actions.json>
"""
import json
import os
import re
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

FIELDS = ["icon", "name", "submenu", "unique-id", "command",
          "description", "range", "patterns"]


def build(spec):
    action = ET.Element("action")
    for field in FIELDS:
        ET.SubElement(action, field).text = spec.get(field, "")
    if spec.get("startup_notify"):
        ET.SubElement(action, "startup-notify")
    for kind in spec["types"]:
        ET.SubElement(action, kind)
    return action


def main():
    specs = json.loads(Path(sys.argv[1]).read_text())
    config = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    path = config / "Thunar" / "uca.xml"

    if path.is_symlink():  # left over from a read-only home-manager link
        path.unlink()
    if path.exists():
        try:
            root = ET.parse(path).getroot()
        except ET.ParseError as err:
            print(f"thunar custom actions: {path} isn't valid XML ({err}), "
                  "left it alone. Fix or delete it, then rebuild.", file=sys.stderr)
            return
    else:
        root = ET.Element("actions")

    changed = []
    for spec in specs:
        uid = spec["unique-id"]
        found = next((a for a in root.findall("action")
                      if (a.findtext("unique-id") or "") == uid), None)
        if found is None:
            root.append(build(spec))
            changed.append(f"added {spec['name']}")
            continue
        cmd = found.find("command")
        current = cmd.text if cmd is not None else ""
        if current != spec["command"] and any(
                re.search(p, current or "") for p in spec["replace_if"]):
            if cmd is None:
                cmd = ET.SubElement(found, "command")
            cmd.text = spec["command"]
            changed.append(f"updated {spec['name']}")
        pat = found.find("patterns")
        if (pat is not None and spec.get("own_patterns")
                and pat.text != spec["patterns"]
                and re.search(spec["own_patterns"], pat.text or "")):
            pat.text = spec["patterns"]
            changed.append(f"updated file types for {spec['name']}")

    if not changed and path.exists():
        return
    ET.indent(root, space="\t")
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".uca-")
    with os.fdopen(fd, "wb") as out:
        out.write(b'<?xml version="1.0" encoding="UTF-8"?>\n')
        out.write(ET.tostring(root, encoding="utf-8"))
        out.write(b"\n")
    os.replace(tmp, path)
    print("thunar custom actions: " + ", ".join(changed or ["created"]))


if __name__ == "__main__":
    main()
