# Screenshots. Print: select, then annotate/upload/copy/save. Super+Shift+s:
# select to the clipboard.
{ config, pkgs, ... }:
let
  # runs inside the freeze
  screenshot-capture = pkgs.writeShellScriptBin "screenshot-capture" ''
    out=$1
    geom=$(${pkgs.slurp}/bin/slurp -d 2>/dev/null)
    if [ -n "$geom" ]; then
      ${pkgs.grim}/bin/grim -g "$geom" "$out" 2>/dev/null || rm -f "$out"
    else
      rm -f "$out"                      # cancelled
    fi
    ${pkgs.procps}/bin/pkill -x wayfreeze 2>/dev/null
    exit 0
  '';

  screenshot-menu = pkgs.writeShellScriptBin "screenshot-menu" ''
    pics=$(${pkgs.xdg-user-dirs}/bin/xdg-user-dir PICTURES 2>/dev/null || echo "$HOME/Pictures")
    dir="$pics/Screenshots/$(date +%Y-%m)"
    mkdir -p "$dir"
    file="$dir/$(date +%Y-%m-%d_%H-%M-%S).png"

    # freeze the screen while selecting
    ${pkgs.wayfreeze}/bin/wayfreeze --hide-cursor \
      --after-freeze-cmd "${screenshot-capture}/bin/screenshot-capture '$file'" \
      >/dev/null 2>&1
    ${pkgs.procps}/bin/pkill -x wayfreeze 2>/dev/null

    [ -s "$file" ] || exit 0            # cancelled, nothing captured

    MENU="${pkgs.wmenu}/bin/wmenu -f 'Inter 14' ${config.my.wmenuColours} -l 6 -p 'shot:'"

    actions='upload (zipline)
upload advanced
copy to clipboard
save only
open
delete'

    menu="annotate
$actions"

    choice=$(printf '%s' "$menu" | eval $MENU)

    if [ "$choice" = "annotate" ]; then
      before=$(stat -c %Y "$file" 2>/dev/null)
      ${pkgs.satty}/bin/satty --filename "$file" --fullscreen \
        --output-filename "$file" --early-exit \
        --copy-command ${pkgs.wl-clipboard}/bin/wl-copy
      after=$(stat -c %Y "$file" 2>/dev/null)
      # ctrl+c in satty only copies: take it from the clipboard
      if [ "$before" = "$after" ]; then
        tmp=$(mktemp -t satty-XXXXXX.png)
        if ${pkgs.wl-clipboard}/bin/wl-paste --type image/png > "$tmp" 2>/dev/null \
           && [ -s "$tmp" ]; then
          mv "$tmp" "$file"
        else
          rm -f "$tmp"
        fi
      fi
      [ -s "$file" ] || exit 0
      choice=$(printf '%s' "$menu" | eval $MENU)
    fi

    case "$choice" in
      "upload (zipline)")  "$HOME/.bin/zipline-upload" "$file" ;;
      "upload advanced")   "$HOME/.bin/zipline-upload" --advanced "$file" ;;
      "copy to clipboard") ${pkgs.wl-clipboard}/bin/wl-copy < "$file" \
                             && ${pkgs.libnotify}/bin/notify-send -c osd "screenshot" "copied" ;;
      "save only")         ${pkgs.libnotify}/bin/notify-send -c osd "screenshot" "saved to $file" ;;
      open)                ${pkgs.xdg-utils}/bin/xdg-open "$file" >/dev/null 2>&1 & ;;
      delete)              rm -f "$file" ;;
      *)                   : ;;
    esac
  '';
in
{
  home.packages = [ screenshot-menu screenshot-capture ];

  wayland.windowManager.sway.config.keybindings = {
    "Mod4+Shift+s" = "exec ${pkgs.grim}/bin/grim -g \"$(${pkgs.slurp}/bin/slurp)\" - | ${pkgs.wl-clipboard}/bin/wl-copy";
    "Print" = "exec screenshot-menu";
  };
}
