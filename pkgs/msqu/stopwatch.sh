# msqu stopwatch: counts up on the bar (green).
#
#   msqu stopwatch start    from 0 (a running one starts again)
#   msqu stopwatch toggle   pause / resume (clicking the block)
#   msqu stopwatch stop     stop, and show the time (right-clicking)
#   msqu stopwatch running  exit 0 if one is going
#
# state file "stopwatch": "run START SO_FAR" or "pause SO_FAR" (seconds;
# START is a unix time; SO_FAR is what was counted before the last pause)

read_state() {
  mode="" t=0 sofar=0
  [ -f "$state/stopwatch" ] && read -r mode t sofar < "$state/stopwatch"
  [ -n "$mode" ]
}

# seconds counted so far
elapsed() {
  if [ "$mode" = run ]; then
    echo $((sofar + $(now) - t))
  else
    echo "$t"
  fi
}

case "${1:-}" in
  start) put stopwatch "run $(now) 0" ;;
  toggle | click)
    read_state || exit 0
    if [ "$mode" = run ]; then
      put stopwatch "pause $(elapsed)"
    else
      put stopwatch "run $(now) $t"
    fi
    ;;
  stop | cancel | right-click)
    read_state || exit 0
    total=$(elapsed)
    rm -f "$state/stopwatch"
    notify-send "stopwatch" "$(hms "$total")"
    ;;
  running) read_state ;;
  middle-click) ;;
  *)
    echo "usage: msqu stopwatch [start|toggle|stop|running]" >&2
    exit 2
    ;;
esac
