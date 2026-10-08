# Related apps join one tabbed group per category when they open. Only
# the opening is handled: afterwards the windows are left alone and can be
# dragged out. Super+Alt+g pauses/resumes grouping.
# Source: ../pkgs/app-group/main.go
{ pkgs, lib, ... }:
let
  # group name -> apps (Wayland app_id or XWayland class, case-insensitive).
  # To check a window: swaymsg -t get_tree | grep -E '"(app_id|class)"'
  groups = {
    chat = [ "vesktop" "signal" "ferdium" "org.telegram.desktop" "TelegramDesktop" ];
    minecraft = [ "org.prismlauncher.PrismLauncher" "com.mojang.minecraft" ];
  };

  appGroup = pkgs.buildGoModule {
    pname = "app-group";
    version = "1.2";
    src = ../pkgs/app-group;
    vendorHash = null;   # standard library only
    ldflags = [ "-s" "-w" ];
  };

  # flips the flag file the watcher checks when a window opens
  toggle = pkgs.writeShellApplication {
    name = "app-group-toggle";
    runtimeInputs = [ pkgs.coreutils pkgs.libnotify ];
    text = ''
      flag="''${XDG_STATE_HOME:-$HOME/.local/state}/app-group/off"
      mkdir -p "$(dirname "$flag")"
      if [ -e "$flag" ]; then
        rm "$flag"; state="on"
      else
        touch "$flag"; state="off"
      fi
      notify-send -c osd -h string:x-canonical-private-synchronous:osd \
        -t 1500 "app grouping" "$state"
    '';
  };

  args = lib.mapAttrsToList
    (name: apps: "${name}=${lib.concatStringsSep "," apps}") groups;
in
{
  systemd.user.services.app-group = {
    Unit = {
      Description = "Group related apps into tabs when they open";
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = "${appGroup}/bin/app-group ${lib.escapeShellArgs args}";
      Restart = "on-failure";
      RestartSec = 3;
    };
    Install.WantedBy = [ "sway-session.target" ];
  };

  wayland.windowManager.sway.config.keybindings."Mod4+Alt+g" =
    "exec ${lib.getExe toggle}";
}
