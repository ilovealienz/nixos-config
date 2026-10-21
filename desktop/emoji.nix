# Super+;: emoji picker. Enter pastes into the focused window (Ctrl+V:
# typing breaks in Electron and XWayland), Shift+Enter copies.
# The list is built from Unicode's data, so bemoji never downloads.
{ config, pkgs, lib, ... }:
let
  emojiDb = pkgs.runCommand "bemoji-emoji-list" { } ''
    mkdir -p $out
    sed -ne 's/^.*; fully-qualified.*# \(\S*\) \S* \(.*$\)/\1 \2/gp' \
      ${pkgs.unicode-emoji}/share/unicode/emoji/emoji-test.txt > $out/emojis.txt
    [ -s $out/emojis.txt ]   # fail on an empty list
  '';

  emojiPicker = pkgs.writeShellApplication {
    name = "emoji-picker";
    runtimeInputs = with pkgs; [ bemoji fuzzel wl-clipboard wtype xdotool libnotify jq sway coreutils gnugrep gnused ];
    text = ''
      export BEMOJI_DB_LOCATION=${emojiDb}
      # grep . drops the blank line bemoji adds before the recent list
      export BEMOJI_PICKER_CMD="grep . | fuzzel --dmenu --config ${config.my.pickerFuzzel} --prompt 'emoji: ' --width 50 --placeholder 'enter: type · shift+enter: copy'"
      # Shift+Enter: prefix with copy: so this script handles it
      export BEMOJI_CLIP_CMD="sed s/^/copy:/"
      export BEMOJI_TYPE_CMD=true   # bemoji's own typing (Alt+2): no-op

      out=$(bemoji -e -n) || exit 0   # -e: print it, -n: no newline
      [ -n "$out" ] || exit 0
      emoji=''${out#copy:}
      printf '%s' "$emoji" | wl-copy

      if [ "$out" != "$emoji" ]; then   # Shift+Enter
        notify-send -c osd -h string:x-canonical-private-synchronous:osd \
          -t 1500 "copied" "$emoji"
        exit 0
      fi

      shell=- app=-
      read -r shell app < <(swaymsg -t get_tree | jq -r '
        first(.. | objects | select(.focused == true))
        | "\(.shell // "-") \(.app_id // .window_properties.class // "-")"') || true
      # let focus return from fuzzel
      sleep 0.15
      if [ "$shell" = xwayland ]; then
        # wtype doesn't reliably reach XWayland
        xdotool key --clearmodifiers ctrl+v
      else
        case "$app" in
          # terminals paste with Ctrl+Shift+V
          kitty|kitty-float|foot|foot-float|footclient|Alacritty|org.wezfurlong.wezterm|com.mitchellh.ghostty)
            wtype -M ctrl -M shift -k v -m shift -m ctrl ;;
          *) wtype -M ctrl -k v -m ctrl ;;
        esac
      fi
    '';
  };
in
{
  wayland.windowManager.sway.config.keybindings."Mod4+semicolon" = "exec ${lib.getExe emojiPicker}";
}
