# Volume, brightness and media keys; levels shown as mako notifications.
{ pkgs, ... }:
let
  osd = pkgs.writeShellScriptBin "osd" ''
    notify() {
      ${pkgs.libnotify}/bin/notify-send -c osd \
        -h int:value:"$2" \
        -h string:x-canonical-private-synchronous:osd \
        -t 1500 "$1" "$2%"
    }

    case "$1" in
      vol-up)
        ${pkgs.wireplumber}/bin/wpctl set-volume -l 1.5 @DEFAULT_AUDIO_SINK@ 5%+
        notify "volume" "$(${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gawk}/bin/awk '{print int($2*100)}')" ;;
      vol-down)
        ${pkgs.wireplumber}/bin/wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-
        notify "volume" "$(${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gawk}/bin/awk '{print int($2*100)}')" ;;
      vol-mute)
        ${pkgs.wireplumber}/bin/wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
        if ${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gnugrep}/bin/grep -q MUTED; then
          ${pkgs.libnotify}/bin/notify-send -c osd -h string:x-canonical-private-synchronous:osd -t 1500 "volume" "muted"
        else
          notify "volume" "$(${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gawk}/bin/awk '{print int($2*100)}')"
        fi ;;
      bright-up)
        ${pkgs.brightnessctl}/bin/brightnessctl set 5%+ >/dev/null
        notify "brightness" "$(${pkgs.brightnessctl}/bin/brightnessctl -m | ${pkgs.coreutils}/bin/cut -d, -f4 | ${pkgs.gnused}/bin/sed 's/%//')" ;;
      bright-down)
        ${pkgs.brightnessctl}/bin/brightnessctl set 5%- >/dev/null
        notify "brightness" "$(${pkgs.brightnessctl}/bin/brightnessctl -m | ${pkgs.coreutils}/bin/cut -d, -f4 | ${pkgs.gnused}/bin/sed 's/%//')" ;;
    esac
  '';
in
{
  home.packages = [ osd ];

  wayland.windowManager.sway.config.keybindings = {
    "XF86AudioRaiseVolume" = "exec osd vol-up";
    "XF86AudioLowerVolume" = "exec osd vol-down";
    "XF86AudioMute" = "exec osd vol-mute";
    "XF86MonBrightnessUp" = "exec osd bright-up";
    "XF86MonBrightnessDown" = "exec osd bright-down";
    "XF86AudioPlay" = "exec ${pkgs.playerctl}/bin/playerctl play-pause";
    "XF86AudioNext" = "exec ${pkgs.playerctl}/bin/playerctl next";
    "XF86AudioPrev" = "exec ${pkgs.playerctl}/bin/playerctl previous";
  };
}
