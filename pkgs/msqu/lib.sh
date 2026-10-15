# msqu's shared helpers. sway/msqu.nix puts this file in front of each
# tool's script, so every tool gets them. A tool's script handles its own
# commands (start, toggle, cancel, ring, ...) and keeps what it's doing in
# a state file under $state, written with `put`; the bar block is
# msqu-bar (menu/src/bin/msqu-bar.rs), which watches that file and draws
# it, and asking what to start is the menu's job (msqu-ui). The state
# files' formats are in each script and in menu/src/model.rs, which reads
# them.
#
# Also here: an alarm that runs the tool's "ring" command at a set time
# (`arm`, `disarm`), the bell (`bell`) and a ringing notification
# (`ring`).
#
# A tool that can run several at once (timer, alarm) has MSQU_SLOTS of
# them, numbered from 1, each with its own state file ("1", "2", ...), bar
# block and alarm. The script sets `slot` to the one it's working on
# (`free_slot` finds one for something new); arm and disarm go by it.
# Tools that run one at a time leave it empty.
#
# MSQU_TOOL (the tool's name), MSQU_BELL, and MSQU_SLOTS and MSQU_ALARM
# (for tools that have them) are set by sway/msqu.nix.

state="${XDG_RUNTIME_DIR:-/tmp}/msqu/$MSQU_TOOL"
mkdir -p "$state"
slot=""

# One command of a tool at a time: two clicks close together (or a click
# while an alarm goes off) would otherwise interleave, and could arm two
# alarms. The lock is kept until the command exits, or `unlock` is called
# before anything slow (the bell, a ringing alarm, fuzzel). It lives
# outside $state so taking it doesn't wake the bar blocks watching that.
exec 9>"$state.lock"
flock 9
unlock() { flock -u 9; }

# the first slot with nothing in it; fails if they're all in use
free_slot() {
  local i
  for ((i = 1; i <= ${MSQU_SLOTS:-1}; i++)); do
    if [ ! -e "$state/$i" ]; then
      echo "$i"
      return 0
    fi
  done
  return 1
}

# exit 0 if any slot has something in it
any_slot() {
  local i
  for ((i = 1; i <= ${MSQU_SLOTS:-1}; i++)); do
    [ -e "$state/$i" ] && return 0
  done
  return 1
}

# use_slot N: work on slot N, if it's one of this tool's
use_slot() {
  if ! [[ ${1:-} =~ ^[1-9][0-9]*$ ]] || [ "$1" -gt "${MSQU_SLOTS:-0}" ]; then
    echo "msqu $MSQU_TOOL: no slot ${1:-(none given)}, there are ${MSQU_SLOTS:-0}" >&2
    exit 2
  fi
  slot=$1
}

# put NAME CONTENT: write a state file in one go, so the bar never reads
# half of it
put() {
  printf '%s\n' "$2" > "$state/.$1.tmp"
  mv -f "$state/.$1.tmp" "$state/$1"
}

# 75 -> 1:15, 3725 -> 1:02:05
hms() {
  local s=$1
  if [ "$s" -ge 3600 ]; then
    printf '%d:%02d:%02d' $((s / 3600)) $((s % 3600 / 60)) $((s % 60))
  else
    printf '%d:%02d' $((s / 60)) $((s % 60))
  fi
}

now() { date +%s; }

# the alarm: runs "THIS-TOOL ring" at unix time $1, as a systemd user timer
# so it survives reloading sway or waybar. A calendar time rather than "in
# N seconds": time spent suspended counts, and an alarm that came due
# while suspended goes off on resume.
# Each alarm gets its own unit, named after its time, and disarm stops only
# timers: a "ring" that arms the next alarm (repeat) runs inside the last
# alarm's service, and stopping that service would kill it halfway.
# With slots, each slot's alarms are its own: msqu-timer-2-UNIXTIME-N.
# (N, nanoseconds, keeps the name new even when re-armed for the same
# second, before systemd has dropped the old unit.)
# The ring gets MSQU_AT, the time it was armed for; `stale` tells a ring
# whose alarm was replaced meanwhile (it went off as a click re-armed it).
unit() { echo "msqu-$MSQU_TOOL${slot:+-$slot}"; }
disarm() {
  systemctl --user stop "$(unit).timer" "$(unit)-*.timer" 2>/dev/null || true
}
arm() {
  disarm
  systemctl --user reset-failed "$(unit)-*" 2>/dev/null || true
  # --collect and RemainAfterElapse=no: the units go once it has rung
  # (successfully or not), rather than staying loaded until the next arm
  systemd-run --user --quiet --collect --unit="$(unit)-$1-$(date +%N)" --setenv=MSQU_AT="$1" \
    --on-calendar="$(date -d "@$1" '+%Y-%m-%d %H:%M:%S')" \
    --timer-property=AccuracySec=1s --timer-property=RemainAfterElapse=no \
    "$(readlink -f "$0")" ring ${slot:+"$slot"}
}

# in "ring", after read_state: true if this ring's alarm isn't the one in
# the state file any more (not running, or armed for another time).
# Rings from before MSQU_AT existed are never stale.
stale() {
  [ -n "${MSQU_AT:-}" ] || return 1
  [[ $mode != run && $mode != at ]] || [ "$t" != "$MSQU_AT" ]
}

# do-not-disturb on (mako's "dnd" mode, the DND icon in waybar)
dnd() {
  local modes
  modes=$(makoctl mode 2>/dev/null) || return 1
  [[ $'\n'$modes$'\n' == *$'\n'dnd$'\n'* ]]
}

# the bell, unless do-not-disturb is on (a timer or alarm's `ring` still
# sounds: it was asked for)
bell() {
  unlock
  if ! dnd; then
    pw-play "$MSQU_BELL" || true
  fi
}

# ring SUMMARY BODY: a notification that stays until closed, with the
# alarm sound (MSQU_ALARM) playing over and over until then. The sound
# gives up after 10 minutes, leaving the notification up.
ring() {
  local loop rc=0
  unlock
  (
    p=""
    trap 'if [ -n "$p" ]; then kill "$p" 2>/dev/null; fi; exit 0' TERM
    while :; do
      pw-play "$MSQU_ALARM" &
      p=$!
      wait "$p" || sleep 1
    done
  ) &
  loop=$!
  # -w: until it's closed; -t 0: mako's 5s default doesn't close it
  timeout 600 notify-send -w -t 0 -u critical -c msqu-ring "$1" "$2" >/dev/null || rc=$?
  kill "$loop" 2>/dev/null || true
  wait "$loop" 2>/dev/null || true
  # it failed rather than timing out (no notification daemon?), so nothing
  # to close: play the sound through once instead
  if [ "$rc" -ne 0 ] && [ "$rc" -ne 124 ]; then
    pw-play "$MSQU_ALARM" || true
  fi
}
