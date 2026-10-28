# msqu (my sway quick utils)
#
#   msqu               the menu
#   msqu menu [TOOL]   the menu, on that tool's form
#   msqu TOOL [args]   run a tool, e.g. msqu stopwatch stop
#
# MSQU_TOOLS and MSQU_MENU come from desktop/msqu.nix.

usage() {
  echo "usage: msqu [menu [TOOL] | TOOL [args]]   tools: $MSQU_TOOLS" >&2
  exit 2
}

case "${1:-menu}" in
  menu)
    shift || true
    # a layer-shell overlay, so it shows over fullscreen windows
    exec kitty +kitten panel --edge=center-sized --layer=overlay --focus-policy=exclusive \
      --columns=660px --lines=460px --app-id=msqu "$MSQU_MENU" --frame "$@"
    ;;
esac

case " $MSQU_TOOLS " in
  *" $1 "*) tool=$1; shift; exec "msqu-$tool" "$@" ;;
  *) usage ;;
esac
