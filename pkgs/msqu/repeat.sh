# msqu repeat: a reminder that goes off every so often until you stop it,
# and can bring a window to the front each time (keeping a game from
# going AFK, say). Counts down on the bar in blue; faint while paused.
#
#   msqu repeat start SECONDS WINDOW [LABEL]
#                         WINDOW: app_id=ID, class=CLASS or - for none
#   msqu repeat windows   the open windows, one per app: "WINDOW<tab>title"
#   msqu repeat toggle    pause / resume (clicking the block)
#   msqu repeat cancel    stop (right-clicking the block)
#
# One repeat at a time: starting another replaces it. Its notifications
# (category msqu-repeat) stay out of the notification history.
#
# state file "repeat": "run END EVERY WINDOW LABEL" or
# "pause LEFT EVERY WINDOW LABEL" (seconds; END is a unix time; WINDOW is
# app_id=ID, class=CLASS or - for none)

read_state() {
  mode="" t=0 every=0 window="-" label=""
  [ -f "$state/repeat" ] && read -r mode t every window label < "$state/repeat"
  [ -n "$mode" ]
}

# "app_id=org.vinegarhq.Sober" -> "org.vinegarhq.Sober"
window_name() { [ "$1" = - ] && echo "nothing" || echo "${1#*=}"; }

start() {
  local secs=$1 win=$2 name=$3 end
  end=$(($(now) + secs))
  arm "$end"
  put repeat "run $end $secs $win $name"
  notify-send -c osd -t 2000 "$name" "every $(hms "$secs")$([ "$win" = - ] || echo ", brings up $(window_name "$win")")"
}

case "${1:-}" in
  start)
    secs=${2:-} win=${3:--} name=${4:-}
    if ! [[ $secs =~ ^[0-9]+$ ]] || [ "$secs" -le 0 ] || [[ $win != - && $win != app_id=* && $win != class=* ]]; then
      echo "usage: msqu repeat start SECONDS WINDOW [LABEL]" >&2
      exit 2
    fi
    [ -n "$name" ] || name=$(window_name "$win")
    [ "$name" = nothing ] && name="reminder"
    start "$secs" "$win" "$name"
    ;;
  windows)
    swaymsg -t get_tree | jq -r '
      [.. | objects | select(.pid? and (.app_id // .window_properties.class))
        | { w: (if .app_id then "app_id=\(.app_id)" else "class=\(.window_properties.class)" end),
            t: ((.name // "") | gsub("\t"; " ")) }]
      | unique_by(.w)[] | "\(.w)\t\(.t)"'
    ;;
  toggle | click)
    read_state || exit 0
    if [ "$mode" = run ]; then
      disarm
      left=$((t - $(now)))
      [ "$left" -lt 1 ] && left=1
      put repeat "pause $left $every $window $label"
    else
      end=$(($(now) + t))
      arm "$end"
      put repeat "run $end $every $window $label"
    fi
    ;;
  cancel | right-click)
    disarm
    rm -f "$state/repeat"
    ;;
  running) read_state ;;
  ring)
    read_state && [ "$mode" = run ] || exit 0
    # the next one first, so it keeps going whatever happens below
    end=$(($(now) + every))
    arm "$end"
    put repeat "run $end $every $window $label"
    if [ "$window" = - ]; then
      notify-send -c msqu-repeat "$label" "every $(hms "$every")"
    elif swaymsg -q "[${window%%=*}=\"${window#*=}\"] focus"; then
      notify-send -c msqu-repeat "$label" "brought up $(window_name "$window")"
    else
      notify-send -c msqu-repeat -u critical "$label" "$(window_name "$window") isn't open"
    fi
    bell
    ;;
  middle-click) ;;
  *)
    echo "usage: msqu repeat [start SECONDS WINDOW [LABEL]|windows|toggle|cancel|running]" >&2
    exit 2
    ;;
esac
