# Screen-share picker for xdg-desktop-portal-wlr that only asks once.
#
# Electron apps (Vesktop, Discord...) open several screencast sessions for
# one share (preview + stream). Each one runs the picker, and all of them
# must get the same answer or the stream never loads. This shows fuzzel
# for the first request, remembers the pick, and answers follow-up
# requests within 60s with the same monitor/window (if it's still on
# offer). Windows are matched by their id, not their title, so a title
# that changes in the meantime (e.g. an unread count) still matches.
# Cancelling (Escape) declines any request that was already waiting.
{ writeShellApplication, fuzzel, coreutils, gawk, util-linux }:

writeShellApplication {
  name = "share-chooser";
  runtimeInputs = [ fuzzel coreutils gawk util-linux ];
  text = ''
    cache="''${XDG_RUNTIME_DIR:-/tmp}/share-chooser-last"
    window=10   # seconds a pick is reused for

    # Identity of an option that survives title changes. xdpw's lines are
    #   "Monitor: <name> <description>"  -> M:<name>
    #   "Window: <title> (<id>)"         -> W:<id>
    keyprog='
      function key(l,  s, f) {
        if (index(l, "Window: ") == 1) { s = l; sub(/.*\(/, "", s); sub(/\)$/, "", s); return "W:" s }
        if (index(l, "Monitor: ") == 1) { split(l, f, " "); return "M:" f[2] }
        return "?:" l
      }'

    list=$(cat)

    # one picker at a time: later requests wait here, then reuse the answer
    exec 9>"$cache.lock"
    flock 9

    if [ -f "$cache" ]; then
      age=$(( $(date +%s) - $(stat -c %Y "$cache") ))
      last=$(cat "$cache")
      # you just cancelled: decline the requests that were waiting too
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
