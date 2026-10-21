# Super+;: emoji picker (bemoji in fuzzel), recent emoji first.
# Enter puts the emoji into the focused window, Shift+Enter only copies
# it. It's pasted rather than typed: wtype types emoji as the wrong
# characters in Electron apps and XWayland windows, but a Ctrl+V
# keypress works everywhere (sent with xdotool to XWayland windows).
# Uses the pickers' fuzzel config (menus.nix), where Shift+Enter is custom-1.
# The emoji list is built from Unicode's data file, so bemoji never downloads
# (a failed download leaves an empty list it never retries).
{ config, pkgs, lib, ... }:
let
  emojiDb = pkgs.runCommand "bemoji-emoji-list" { } ''
    mkdir -p $out
    sed -ne 's/^.*; fully-qualified.*# \(\S*\) \S* \(.*$\)/\1 \2/gp' \
      ${pkgs.unicode-emoji}/share/unicode/emoji/emoji-test.txt > $out/emojis.txt
    [ -s $out/emojis.txt ]   # fail the build rather than ship an empty list
  '';

  emojiPicker = pkgs.writeShellApplication {
    name = "emoji-picker";
    runtimeInputs = with pkgs; [ bemoji fuzzel wl-clipboard wtype xdotool libnotify jq sway coreutils gnugrep gnused ];
    text = ''
      export BEMOJI_DB_LOCATION=${emojiDb}
      # grep . drops the blank line bemoji adds before the recent list
      export BEMOJI_PICKER_CMD="grep . | fuzzel --dmenu --config ${config.my.pickerFuzzel} --prompt 'emoji: ' --width 50 --placeholder 'enter: type · shift+enter: copy'"
      # bemoji copies on fuzzel's custom-1 (Shift+Enter); make that print
      # the emoji with a "copy:" prefix instead, so this script handles it
      export BEMOJI_CLIP_CMD="sed s/^/copy:/"
      export BEMOJI_TYPE_CMD=true   # bemoji's own typing (Alt+2): no-op

      out=$(bemoji -e -n) || exit 0   # -e: print it, -n: no newline
      [ -n "$out" ] || exit 0
      emoji=''${out#copy:}
      printf '%s' "$emoji" | wl-copy

      if [ "$out" != "$emoji" ]; then   # Shift+Enter
        # osd category: not kept in mako's history
        notify-send -c osd -h string:x-canonical-private-synchronous:osd \
          -t 1500 "copied" "$emoji"
        exit 0
      fi

      shell=- app=-
      read -r shell app < <(swaymsg -t get_tree | jq -r '
        first(.. | objects | select(.focused == true))
        | "\(.shell // "-") \(.app_id // .window_properties.class // "-")"') || true
      # let keyboard focus return from fuzzel; XWayland takes a moment
      # longer to hand X focus back to the window
      sleep 0.15
      if [ "$shell" = xwayland ]; then
        # X11 windows (Proton, Bottles/Wine, Brave): send the key through
        # the X server; wtype's keys don't reliably reach XWayland
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
