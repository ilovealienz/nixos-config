# msqu (my sway quick utils): small tools with a menu on Super+Shift+A and,
# if they want one, a block on the bar.
#
#   msqu                     the menu, in a floating terminal
#   msqu menu [TOOL]         the menu, open on that tool's form (timer,
#                            alarm, repeat or drink)
#   msqu TOOL [args]         run a tool, e.g. msqu stopwatch stop
#
# The menu is menu/ (msqu-ui, Rust); each tool is a script next to this
# one (timer.sh) with lib.sh's helpers. MSQU_TOOLS and MSQU_MENU are set
# by sway/msqu.nix.

usage() {
  echo "usage: msqu [menu [TOOL] | TOOL [args]]   tools: $MSQU_TOOLS" >&2
  exit 2
}

case "${1:-menu}" in
  menu)
    shift || true
    # its own app_id, so sway floats and sizes it (sway/msqu.nix)
    exec kitty --class msqu -e "$MSQU_MENU" "$@"
    ;;
esac

case " $MSQU_TOOLS " in
  *" $1 "*) tool=$1; shift; exec "msqu-$tool" "$@" ;;
  *) usage ;;
esac
