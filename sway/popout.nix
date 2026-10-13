# Super+Shift+V: pop the focused window out into a small window in the
# bottom-right corner that stays on top and follows you to every workspace
# on that monitor (a video, a call). Press it again on the popout to put it
# back in the tiling; if it's drifted out of the corner, the press puts
# it back in the corner first.
# Brave's own picture-in-picture window is popped out the same way when it
# opens.
# Sway draws it over everything except a fullscreen window on the same
# monitor; fullscreen on another monitor is fine.
{ pkgs, lib, ... }:
let
  popout = pkgs.writeShellApplication {
    name = "popout";
    runtimeInputs = with pkgs; [ sway jq ];
    text = ''
      tree=$(swaymsg -t get_tree)
      # a window the picture-in-picture rule just marked, else the focused
      # one, with the workspace it's on
      read -r id type sticky marked x y w h wx wy ww wh < <(jq -r '
        def wins: .. | objects | select(.type? == "workspace") as $ws
          | $ws | .. | objects | {c: ., ws: $ws.rect};
        (first(wins | select(.c.marks? // [] | index("_popout_new")))
          // first(wins | select(.c.focused? == true)))
        | [.c.id, .c.type, .c.sticky, (.c.marks // [] | index("_popout_new") != null),
           .c.rect.x, .c.rect.y, .c.rect.width, .c.rect.height,
           .ws.x, .ws.y, .ws.width, .ws.height]
        | @tsv' <<<"$tree")
      [ -n "''${id:-}" ] || exit 0
      [ "$type" = workspace ] && exit 0
      m=16
      corner() { echo "move position $((ww - $1 - m)) px $((wh - $2 - m)) px"; }
      if [ "$marked" = true ]; then
        swaymsg -q "[con_id=$id] unmark _popout_new"
      elif [ "$type" = floating_con ] && [ "$sticky" = true ]; then
        # already popped out: back to the tiling if it's in the corner,
        # else back to the corner (an X11 window that resizes itself, like
        # Brave's picture-in-picture when the video changes shape, gets
        # re-centred by sway)
        dx=$((wx + ww - m - x - w)); dy=$((wy + wh - m - y - h))
        if [ "''${dx#-}" -le 4 ] && [ "''${dy#-}" -le 4 ]; then
          swaymsg -q "[con_id=$id] sticky disable, floating disable"
        else
          swaymsg -q "[con_id=$id] $(corner "$w" "$h")"
        fi
        exit 0
      fi
      # a quarter of the screen wide, 16:9, 16px from the corner
      w=$((ww / 4)); h=$((w * 9 / 16))
      swaymsg -q "[con_id=$id] fullscreen disable, floating enable, sticky enable, resize set $w px $h px, $(corner "$w" "$h")"
    '';
  };
in
{
  home.packages = [ popout ];
  wayland.windowManager.sway.config = {
    keybindings."Mod4+Shift+v" = "exec ${lib.getExe popout}";
    # Brave/Chromium's picture-in-picture window (Firefox's is titled
    # Picture-in-Picture)
    window.commands = [{
      criteria.title = "^Picture[ -]in[ -][Pp]icture$";
      command = "floating enable, sticky enable, mark --add _popout_new, exec ${lib.getExe popout}";
    }];
  };
}
