# Super+Alt+V: pop the window out to a small sticky window in the bottom-right
# corner; again to put it back (or back in the corner if it drifted).
# Browser picture-in-picture windows pop out automatically.
{ pkgs, lib, ... }:
let
  popout = pkgs.writeShellApplication {
    name = "popout";
    runtimeInputs = with pkgs; [ sway jq ];
    text = ''
      tree=$(swaymsg -t get_tree)
      # a just-marked picture-in-picture window, else the focused one, and
      # the current workspace
      read -r id type sticky marked x y w h wx wy ww wh wsname < <(jq -r '
        def wins: .. | objects | select(.type? == "workspace") as $ws
          | $ws | .. | objects | {c: ., ws: $ws};
        (first(wins | select(.c.focused? == true)) | .ws) as $f
        | (first(wins | select(.c.marks? // [] | index("_popout_new")))
            // first(wins | select(.c.focused? == true)))
        | [.c.id, .c.type, .c.sticky, (.c.marks // [] | index("_popout_new") != null),
           .c.rect.x, .c.rect.y, .c.rect.width, .c.rect.height,
           $f.rect.x, $f.rect.y, $f.rect.width, $f.rect.height, $f.name]
        | @tsv' <<<"$tree")
      [ -n "''${id:-}" ] || exit 0
      [ "$type" = workspace ] && exit 0
      m=16
      corner() { echo "move position $((ww - $1 - m)) px $((wh - $2 - m)) px"; }
      if [ "$marked" = true ]; then
        swaymsg -q "[con_id=$id] unmark _popout_new, move container to workspace \"$wsname\""
      elif [ "$type" = floating_con ] && [ "$sticky" = true ]; then
        # popped out: back to tiling if in the corner, else to the corner
        dx=$((wx + ww - m - x - w)); dy=$((wy + wh - m - y - h))
        if [ "''${dx#-}" -le 4 ] && [ "''${dy#-}" -le 4 ]; then
          swaymsg -q "[con_id=$id] sticky disable, floating disable"
        else
          swaymsg -q "[con_id=$id] $(corner "$w" "$h")"
        fi
        exit 0
      fi
      # quarter screen width, 16:9
      w=$((ww / 4)); h=$((w * 9 / 16))
      swaymsg -q "[con_id=$id] fullscreen disable, floating enable, sticky enable, resize set $w px $h px, $(corner "$w" "$h")"
    '';
  };
in
{
  home.packages = [ popout ];
  wayland.windowManager.sway.config = {
    keybindings."Mod4+Alt+v" = "exec ${lib.getExe popout}";
    # picture-in-picture windows (Brave, LibreWolf)
    window.commands = [{
      criteria.title = "^Picture[ -]in[ -][Pp]icture$";
      command = "floating enable, sticky enable, mark --add _popout_new, exec ${lib.getExe popout}";
    }];
  };
}
