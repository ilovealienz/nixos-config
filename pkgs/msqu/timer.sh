# msqu timer: countdowns on the bar (yellow; faint while paused), up to
# MSQU_SLOTS at once, each its own block. When one ends, a notification
# shows and the alarm sound plays until it's closed.
#
#   msqu timer start SECONDS [LABEL]   in the first free slot
#   msqu timer toggle SLOT   pause / resume (clicking its block)
#   msqu timer cancel SLOT   (right-clicking its block)
#   msqu timer running       exit 0 if any timer is set
#
# state file per slot ("1", "2", ...): "run END TOTAL LABEL" or
# "pause LEFT TOTAL LABEL" (seconds; END is a unix time)

read_state() {
  mode="" t=0 total=0 label=""
  [ -n "$slot" ] && [ -f "$state/$slot" ] && read -r mode t total label < "$state/$slot"
  [ -n "$mode" ]
}

case "${1:-}" in
  start)
    secs=${2:-} label=${3:-}
    if ! [[ $secs =~ ^[0-9]+$ ]] || [ "$secs" -le 0 ]; then
      echo "usage: msqu timer start SECONDS [LABEL]" >&2
      exit 2
    fi
    if ! slot=$(free_slot); then
      echo "$MSQU_SLOTS timers are running already: cancel one first" >&2
      exit 1
    fi
    end=$(($(now) + secs))
    arm "$end"
    put "$slot" "run $end $secs $label"
    notify-send -c osd -t 1500 "timer" "$(hms "$secs")${label:+: $label}"
    ;;
  toggle | click)
    use_slot "${2:-}"
    read_state || exit 0
    if [ "$mode" = run ]; then
      disarm
      left=$((t - $(now)))
      [ "$left" -lt 1 ] && left=1
      put "$slot" "pause $left $total $label"
    else
      end=$(($(now) + t))
      arm "$end"
      put "$slot" "run $end $total $label"
    fi
    ;;
  cancel | right-click)
    use_slot "${2:-}"
    disarm
    rm -f "$state/$slot"
    ;;
  running) any_slot ;;
  ring)
    # no slot: one set before timers had slots (its state file is
    # "timer"; nothing reads it now)
    [ -n "${2:-}" ] && use_slot "$2"
    msg="done"
    read_state && msg="${label:-done} ($(hms "$total"))"
    if [ -n "$slot" ]; then
      rm -f "$state/$slot"
    else
      rm -f "$state/$MSQU_TOOL"
    fi
    ring "timer" "$msg"
    ;;
  middle-click) ;;
  *)
    echo "usage: msqu timer [start SECONDS [LABEL]|toggle SLOT|cancel SLOT|running]" >&2
    exit 2
    ;;
esac
