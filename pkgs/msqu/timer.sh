# msqu timer: countdowns on the bar (yellow), up to
# MSQU_SLOTS at once, each its own block. When one ends, a notification
# shows and the alarm sound plays until it's closed.
#
#   msqu timer start SECONDS [LABEL]   in the first free slot
#   msqu timer toggle SLOT   pause / resume (clicking its block)
#   msqu timer cancel SLOT   (right-clicking its block)
#   msqu timer add SLOT AMOUNT   more time, or less: +5m, -1m, 90s, 1:30
#                            (a plain number is minutes)
#   msqu timer more SLOT     pick how much in fuzzel (middle-clicking its
#                            block)
#   msqu timer running       exit 0 if any timer is set
#
# state file per slot ("1", "2", ...): "run END TOTAL LABEL" or
# "pause LEFT TOTAL LABEL" (seconds; END is a unix time)

read_state() {
  mode="" t=0 total=0 label=""
  [ -n "$slot" ] && [ -f "$state/$slot" ] && read -r mode t total label < "$state/$slot"
  [ -n "$mode" ]
}

# "+5m" "-1m" "3" (minutes) "90s" "1h" "1:30" -> seconds, signed; fails on
# anything else
signed_secs() {
  local s=${1// /} sign=1 n
  case $s in
    +*) s=${s#+} ;;
    -*) s=${s#-} sign=-1 ;;
  esac
  if [[ $s =~ ^([0-9]+):([0-5][0-9])$ ]]; then
    n=$((10#${BASH_REMATCH[1]} * 60 + 10#${BASH_REMATCH[2]}))
  elif [[ $s =~ ^([0-9]+)(m|min)?$ ]]; then
    n=$((10#${BASH_REMATCH[1]} * 60))
  elif [[ $s =~ ^([0-9]+)s$ ]]; then
    n=$((10#${BASH_REMATCH[1]}))
  elif [[ $s =~ ^([0-9]+)h$ ]]; then
    n=$((10#${BASH_REMATCH[1]} * 3600))
  else
    return 1
  fi
  echo $((sign * n))
}

# add SECONDS to the timer in $slot (negative takes time off, down to
# a second left); its total changes the same way
add() {
  local d=$1 left end
  read_state || return 0
  if [ "$mode" = run ]; then
    left=$((t - $(now) + d))
  else
    left=$((t + d))
  fi
  [ "$left" -lt 1 ] && left=1
  total=$((total + d))
  [ "$total" -lt "$left" ] && total=$left
  if [ "$mode" = run ]; then
    end=$(($(now) + left))
    arm "$end"
    put "$slot" "run $end $total $label"
  else
    put "$slot" "pause $left $total $label"
  fi
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
  add)
    use_slot "${2:-}"
    if ! d=$(signed_secs "${3:-}"); then
      echo "usage: msqu timer add SLOT AMOUNT   (+5m, -1m, 90s, 1:30)" >&2
      exit 2
    fi
    add "$d"
    ;;
  more | middle-click)
    use_slot "${2:-}"
    read_state || exit 0
    if [ "$mode" = run ]; then left=$((t - $(now))); else left=$t; fi
    pick=$(printf '%s\n' +1m +5m +10m +15m -1m -5m |
      fuzzel --dmenu --lines 6 --width 34 --prompt "${label:-timer} $(hms "$left")  " \
        --placeholder "or type: +3, -2, 90s, 1:30") || exit 0
    [ -n "$pick" ] || exit 0
    if d=$(signed_secs "$pick"); then
      add "$d"
    else
      notify-send -c osd -t 2000 "timer" "can't read \"$pick\": try +3, -2, 90s or 1:30"
    fi
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
  *)
    echo "usage: msqu timer [start SECONDS [LABEL]|toggle SLOT|cancel SLOT|add SLOT AMOUNT|more SLOT|running]" >&2
    exit 2
    ;;
esac
