# Power menu (Super+Shift+P, or the bar's battery): power profile and
# battery charge limit.
{ pkgs, lib, ... }:
let
  powerMenu = pkgs.writeShellApplication {
    name = "power-menu";
    runtimeInputs = with pkgs; [ power-profiles-daemon upower systemd fuzzel libnotify gnused gnugrep coreutils ];
    text = ''
      notify() {
        notify-send -c osd -h string:x-canonical-private-synchronous:osd -t 1500 "$@"
      }
      # "b true" -> true, "u 90" -> 90
      prop() {
        busctl get-property org.freedesktop.UPower "$bat" org.freedesktop.UPower.Device "$1" 2>/dev/null \
          | cut -d' ' -f2
      }

      current=$(powerprofilesctl get)
      mapfile -t profiles < <(powerprofilesctl list | sed -n 's/^[* ] \([a-z-]*\):$/\1/p')

      rows=()
      for p in "''${profiles[@]}"; do
        if [ "$p" = "$current" ]; then rows+=("● $p"); else rows+=("  $p"); fi
      done

      # only if the battery supports a limit
      bat=$(upower -e 2>/dev/null | grep -m1 '/battery_' || true)
      limit=""
      if [ -n "$bat" ] && [ "$(prop ChargeThresholdSupported)" = true ]; then
        limit=$(prop ChargeThresholdEnabled)
        end=$(prop ChargeEndThreshold)
        if [ "$limit" = true ]; then
          rows+=("  battery limit: on (stops at ''${end}%)")
        else
          rows+=("  battery limit: off (charges to 100%)")
        fi
      fi

      i=$(printf '%s\n' "''${rows[@]}" | fuzzel --dmenu --index --prompt 'power: ' \
            --lines "''${#rows[@]}" --width 36) || exit 0
      [[ $i =~ ^[0-9]+$ ]] && (( i < ''${#rows[@]} )) || exit 0

      if (( i < ''${#profiles[@]} )); then
        p=''${profiles[$i]}
        [ "$p" != "$current" ] || exit 0
        powerprofilesctl set "$p"
        notify "power profile" "$p"
      else
        want=true
        [ "$limit" = true ] && want=false
        busctl call org.freedesktop.UPower "$bat" org.freedesktop.UPower.Device \
          EnableChargeThreshold b "$want"
        if [ "$want" = true ]; then
          notify "battery limit" "on: stops at ''${end}%"
        else
          notify "battery limit" "off: charges to 100%"
        fi
      fi
    '';
  };
in
{
  home.packages = [ powerMenu ];

  wayland.windowManager.sway.config.keybindings."Mod4+Shift+p" =
    "exec ${lib.getExe powerMenu}";
}
