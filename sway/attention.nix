# XWayland apps (Wine/IMVU, Steam chats...) ask for attention with
# _NET_WM_STATE_DEMANDS_ATTENTION, which sway ignores. This small Go watcher
# catches those requests and shows the count in the window's title,
# e.g. "(3) the rock - IMVU", reset when you focus the window.
# Super+U toggles also marking them urgent (red border + red workspace).
# Event-driven: it sleeps until an X11 window sends a request (~2 MB RAM).
# Source: ./xwayland-attention/main.go
{ pkgs, ... }:
let
  # added after the title in every title we set; keep it the same as the
  # `title_format "%title [XWayland]"` rule in compositor.nix
  titleSuffix = " [XWayland]";

  attention = pkgs.buildGoModule {
    pname = "xwayland-attention";
    version = "1.2";
    src = ./xwayland-attention;
    vendorHash = "sha256-YNTxMr9aznzTl0a17XpfDd7Xa4NLbnH8TSwCoTY4IoE=";
    ldflags = [ "-s" "-w" ];
  };

  # flips the flag file the watcher checks on every request
  urgentToggle = pkgs.writeShellApplication {
    name = "attention-urgent-toggle";
    runtimeInputs = [ pkgs.coreutils pkgs.libnotify ];
    text = ''
      flag="''${XDG_STATE_HOME:-$HOME/.local/state}/xwayland-attention/urgent"
      mkdir -p "$(dirname "$flag")"
      if [ -e "$flag" ]; then
        rm "$flag"; state="off"
      else
        touch "$flag"; state="on"
      fi
      notify-send -c osd -h string:x-canonical-private-synchronous:osd \
        -t 1500 "xwayland urgent" "$state"
    '';
  };
in
{
  systemd.user.services.xwayland-attention = {
    Unit = {
      Description = "Count XWayland attention requests in window titles";
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = ''${attention}/bin/xwayland-attention "${titleSuffix}"'';
      Restart = "on-failure";
      RestartSec = 3;
    };
    Install.WantedBy = [ "sway-session.target" ];
  };

  wayland.windowManager.sway.config.keybindings."Mod4+u" =
    "exec ${pkgs.lib.getExe urgentToggle}";
}
