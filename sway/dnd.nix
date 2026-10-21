# Do not disturb: the bell block in waybar. Click for missed notifications,
# right-click to turn do not disturb on / off (mako's dnd mode, mako.nix).
{ config, pkgs, osConfig, ... }:
let
  c = osConfig.theme.colors;

  dnd = pkgs.writeShellScriptBin "dnd" ''
    case "$1" in
      toggle)
        ${pkgs.mako}/bin/makoctl mode -t dnd >/dev/null
        pkill -RTMIN+9 waybar
        ;;
      status)
        if ${pkgs.mako}/bin/makoctl mode | ${pkgs.gnugrep}/bin/grep -q '^dnd$'; then
	  printf '{"text":"\Uf009a","class":"dnd","tooltip":"do not disturb"}\n'
        else
          printf '{"text":"\Uf009a","class":"active","tooltip":"notifications on"}\n'
        fi
        ;;
       history)
        choice=$( { printf 'clear all\n'; \
          ${pkgs.mako}/bin/makoctl history -j \
          | ${pkgs.jq}/bin/jq -r '.[] | "\(.id) \(.app_name): \(.summary) — \(.body)"'; } \
          | ${pkgs.wmenu}/bin/wmenu -f 'Inter 13' ${config.my.wmenuColours} -l 10 -p "missed:" )
        [ "$choice" = "clear all" ] && pkill -f 'bin/mako$'
        ;;
    esac
  '';
in
{
  home.packages = [ dnd ];

  # where it sits in the bar: modules-right in waybar.nix
  programs.waybar.settings.mainBar."custom/dnd" = {
    return-type = "json";
    interval = 5;
    signal = 9;
    exec = "dnd status";
    on-click = "dnd history";
    on-click-right = "dnd toggle";
  };

  programs.waybar.style = ''
    #custom-dnd {
      padding: 0 8px;
      color: #${c.fg};
    }
    #custom-dnd.dnd {
      color: #${c.red};
      background: alpha(#${c.red}, 0.15);
    }
  '';
}
