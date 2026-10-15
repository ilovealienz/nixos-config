# msqu drink: a reminder to drink water every so often, until stopped,
# with repeat's bell. On the bar in magenta, showing
# the minutes to the next one. Skipped while the screen is locked, so you
# don't come back to a pile of them; the next one still comes on time.
#
#   msqu drink start SECONDS
#   msqu drink toggle   pause / resume (clicking the block)
#   msqu drink stop     (right-clicking the block)
#   msqu drink running  exit 0 if it's on
#
# state file "drink": "run END EVERY" or "pause LEFT EVERY"

read_state() {
  mode="" t=0 every=0
  [ -f "$state/drink" ] && read -r mode t every < "$state/drink"
  [ -n "$mode" ]
}

case "${1:-}" in
  start)
    secs=${2:-}
    if ! [[ $secs =~ ^[0-9]+$ ]] || [ "$secs" -le 0 ]; then
      echo "usage: msqu drink start SECONDS" >&2
      exit 2
    fi
    end=$(($(now) + secs))
    arm "$end"
    put drink "run $end $secs"
    notify-send -c osd -t 2000 "drink reminder" "every $(hms "$secs")"
    ;;
  toggle | click)
    read_state || exit 0
    if [ "$mode" = run ]; then
      disarm
      left=$((t - $(now)))
      [ "$left" -lt 1 ] && left=1
      put drink "pause $left $every"
    else
      end=$(($(now) + t))
      arm "$end"
      put drink "run $end $every"
    fi
    ;;
  stop | cancel | right-click)
    disarm
    rm -f "$state/drink"
    ;;
  running) read_state ;;
  ring)
    read_state && [ "$mode" = run ] || exit 0
    end=$(($(now) + every))
    arm "$end"
    put drink "run $end $every"
    # locked: swaylock, or its Nix wrapper's name if it ever gets one
    pgrep -x 'swaylock|\.swaylock-wrapp' >/dev/null && exit 0
    notify-send -c msqu-drink "drink some water" "it's been $(hms "$every")"
    bell
    ;;
  middle-click) ;;
  *)
    echo "usage: msqu drink [start SECONDS|toggle|stop|running]" >&2
    exit 2
    ;;
esac
