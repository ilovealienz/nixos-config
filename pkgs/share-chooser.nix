# Screen-share picker that asks once: Electron apps open several sessions per
# share, so the pick is reused for 10s (windows matched by id).
{ writeShellApplication, fuzzel, coreutils, gawk, util-linux }:

writeShellApplication {
  name = "share-chooser";
  runtimeInputs = [ fuzzel coreutils gawk util-linux ];
  text = ''
    cache="''${XDG_RUNTIME_DIR:-/tmp}/share-chooser-last"
    window=10   # seconds a pick is reused for

    # "Monitor: <name> ..." -> M:<name>, "Window: <title> (<id>)" -> W:<id>
    keyprog='
      function key(l,  s, f) {
        if (index(l, "Window: ") == 1) { s = l; sub(/.*\(/, "", s); sub(/\)$/, "", s); return "W:" s }
        if (index(l, "Monitor: ") == 1) { split(l, f, " "); return "M:" f[2] }
        return "?:" l
      }'

    list=$(cat)

    # one picker at a time
    exec 9>"$cache.lock"
    flock 9

    if [ -f "$cache" ]; then
      age=$(( $(date +%s) - $(stat -c %Y "$cache") ))
      last=$(cat "$cache")
      # just cancelled: decline the waiting ones too
      if [ "$last" = "<cancelled>" ] && [ "$age" -lt 5 ]; then
        exit 1
      fi
      if [ "$age" -lt "$window" ]; then
        match=$(want="$last" awk "$keyprog"' key($0) == ENVIRON["want"] { print; exit }' <<< "$list")
        if [ -n "$match" ]; then
          printf '%s\n' "$match"
          exit 0
        fi
      fi
    fi

    if choice=$(printf '%s\n' "$list" | fuzzel --dmenu --prompt 'share: ') && [ -n "$choice" ]; then
      awk "$keyprog"' { print key($0) }' <<< "$choice" > "$cache"
      printf '%s\n' "$choice"
    else
      printf '%s' "<cancelled>" > "$cache"
      exit 1
    fi
  '';
}
