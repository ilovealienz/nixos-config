# nxdisplay: arrange displays for this session only (nwg-displays); its
# layout file goes to the runtime dir, so monitors/*.nix stays the source
[ -n "${SWAYSOCK:-}" ] || { echo "nxdisplay: needs a sway session" >&2; exit 1; }
out="${XDG_RUNTIME_DIR:-/tmp}/nwg-displays-outputs"

# the current layout, so its revert timer can undo the first change too
swaymsg -r -t get_outputs | jq -r '.[] | if .active then
  "output \"\(.name)\" {\n  mode \(.current_mode.width)x\(.current_mode.height)@\(.current_mode.refresh / 1000)Hz\n  pos \(.rect.x) \(.rect.y)\n  transform \(.transform)\n  scale \(.scale)\n}"
else
  "output \"\(.name)\" {\n  disable\n}"
end' > "$out"

setsid -f nwg-displays -o "$out" >/dev/null 2>&1
