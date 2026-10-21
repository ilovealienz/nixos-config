# App -> workspace placement, toggled with Super+Alt+p.
# The rules live in ~/.config/sway/placement-on.conf. Sway includes
# ~/.local/state/sway/placement.conf, a symlink to that file or to the
# empty placement-off.conf; placement-toggle flips it and reloads sway.
# Matches are the whole name (so kitty doesn't also catch kitty-float),
# in any case (Thunar's app_id is "Thunar").
# Check app ids with: swaymsg -t get_tree | grep -E 'app_id|class'
# (app_id for wayland apps, class for xwayland)
{ config, pkgs, lib, ... }:
let
  placementRules = {
    "1" = [ { app_id = "firefox"; } { app_id = "brave-browser"; } { class = "Brave-browser"; } { app_id = "librewolf"; } { app_id = "LibreWolf"; } ];
    "2" = [ { app_id = "spotify"; } { app_id = "signal"; } { app_id = "vesktop"; } ];
    "3" = [ { app_id = "mpv"; } ];
    "4" = [ { app_id = "kitty"; } { app_id = "foot"; } ];
    "5" = [ { app_id = "thunar"; } ];
    "6" = [ { app_id = "org.qbittorrent.qBittorrent"; } ];
    "7" = [ { app_id = "virt-manager"; } ];
  };
  placementConf = lib.concatStrings (lib.flatten (lib.mapAttrsToList (ws: rules:
    map (r: "assign [${lib.concatStringsSep " "
      (lib.mapAttrsToList (k: v: "${k}=\"(?i)^${v}$\"") r)}] workspace number ${ws}\n") rules)
    placementRules));

  placementToggle = pkgs.writeShellApplication {
    name = "placement-toggle";
    runtimeInputs = [ pkgs.coreutils pkgs.sway pkgs.libnotify ];
    text = ''
      link="$HOME/.local/state/sway/placement.conf"
      cfg="$HOME/.config/sway"
      mkdir -p "$(dirname "$link")"
      if [ "$(readlink "$link" || true)" = "$cfg/placement-off.conf" ]; then
        state=on
      else
        state=off
      fi
      ln -sfn "$cfg/placement-$state.conf" "$link"
      swaymsg -q reload
      notify-send -c osd -h string:x-canonical-private-synchronous:osd \
        -t 1500 "window placement" "$state"
    '';
  };

  # switch to a workspace only while placement is on (Super+Return, Super+e)
  placementFollow = pkgs.writeShellApplication {
    name = "placement-follow";
    runtimeInputs = [ pkgs.coreutils pkgs.sway ];
    text = ''
      [ "$(readlink "$HOME/.local/state/sway/placement.conf" || true)" = "$HOME/.config/sway/placement-off.conf" ] && exit 0
      swaymsg -q "workspace number $1"
    '';
  };
in
{
  # used by the launch keys in sway.nix
  options.my.placementFollow = lib.mkOption {
    type = lib.types.package;
    readOnly = true;
    description = "switches to a workspace only while placement is on";
  };

  config = {
    my.placementFollow = placementFollow;

    xdg.configFile."sway/placement-on.conf".text = placementConf;
    xdg.configFile."sway/placement-off.conf".text = "# window placement off\n";
    # placement starts on; the toggle keeps its choice across rebuilds
    home.activation.swayPlacement = lib.hm.dag.entryAfter [ "linkGeneration" ] ''
      mkdir -p "$HOME/.local/state/sway"
      if [ ! -L "$HOME/.local/state/sway/placement.conf" ]; then
        ln -sfn "$HOME/.config/sway/placement-on.conf" "$HOME/.local/state/sway/placement.conf"
      fi
    '';

    wayland.windowManager.sway.config.keybindings."Mod4+Alt+p" = "exec ${lib.getExe placementToggle}";

    wayland.windowManager.sway.extraConfig = ''
      # app -> workspace rules, toggled with Super+Alt+p
      include $HOME/.local/state/sway/placement.conf
    '';
  };
}
