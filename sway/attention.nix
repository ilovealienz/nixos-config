# XWayland apps (Wine/IMVU, Steam chats...) ask for attention with
# _NET_WM_STATE_DEMANDS_ATTENTION, which sway ignores. This small Go watcher
# catches those requests and marks the window urgent in sway instead,
# so it gets the red border and waybar highlights its workspace.
# Event-driven: it sleeps until an X11 window sends a request (~3 MB RAM).
# Source: ./xwayland-attention/main.go
{ pkgs, ... }:
let
  attention = pkgs.buildGoModule {
    pname = "xwayland-attention";
    version = "1.0";
    src = ./xwayland-attention;
    vendorHash = "sha256-YNTxMr9aznzTl0a17XpfDd7Xa4NLbnH8TSwCoTY4IoE=";
    ldflags = [ "-s" "-w" ];
  };
in
{
  systemd.user.services.xwayland-attention = {
    Unit = {
      Description = "Mark XWayland windows urgent when they ask for attention";
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = "${attention}/bin/xwayland-attention";
      Restart = "on-failure";
      RestartSec = 3;
    };
    Install.WantedBy = [ "sway-session.target" ];
  };
}
