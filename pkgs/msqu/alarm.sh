# msqu alarm: goes off at a time of day (the menu works out the next one),
# up to MSQU_SLOTS at once, each its own block. On the bar in orange,
# showing when. When it goes off, a notification shows and the alarm
# sound plays until it's closed.
#
#   msqu alarm start UNIXTIME [LABEL [SLOT]]
#                            in the first free slot, or in place of SLOT's
#   msqu alarm cancel SLOT   (right-clicking its block)
#   msqu alarm running       exit 0 if any alarm is set
#
# state file per slot ("1", "2", ...): "at UNIXTIME LABEL"

read_state() {
  mode="" t=0 label=""
  [ -n "$slot" ] && [ -f "$state/$slot" ] && read -r mode t label < "$state/$slot"
  [ -n "$mode" ]
}

case "${1:-}" in
  start)
    at=${2:-} label=${3:-}
    if ! [[ $at =~ ^[0-9]+$ ]] || [ "$at" -le "$(now)" ]; then
      echo "usage: msqu alarm start UNIXTIME [LABEL] (a time still to come)" >&2
      exit 2
    fi
    if [ -n "${4:-}" ]; then
      use_slot "$4"
    elif ! slot=$(free_slot); then
      echo "$MSQU_SLOTS alarms are set already: cancel one first" >&2
      exit 1
    fi
    arm "$at"
    put "$slot" "at $at $label"
    notify-send -c osd -t 2000 "alarm" "$(date -d "@$at" '+%H:%M')${label:+: $label}"
    ;;
  cancel | right-click)
    use_slot "${2:-}"
    disarm
    rm -f "$state/$slot"
    ;;
  running) any_slot ;;
  ring)
    # no slot: one set before alarms had slots (its state file is "alarm")
    [ -n "${2:-}" ] && use_slot "$2"
    msg="alarm"
    read_state && msg="${label:-alarm} ($(date -d "@$t" '+%H:%M'))"
    if [ -n "$slot" ]; then
      rm -f "$state/$slot"
    else
      rm -f "$state/$MSQU_TOOL"
    fi
    ring "alarm" "$msg"
    ;;
  click | middle-click) ;;
  *)
    echo "usage: msqu alarm [start UNIXTIME [LABEL [SLOT]]|cancel SLOT|running]" >&2
    exit 2
    ;;
esac
