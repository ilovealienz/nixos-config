{ lib, osConfig, ... }:
let
  isPc = osConfig.networking.hostName == "pc";
in
lib.mkIf isPc {
  wayland.windowManager.hyprland.settings = {
    monitor = [
      "DP-1,1920x1080@143.86,1920x1440,1"
      "DP-2,1920x1080@165,0x1440,1"
      "HDMI-A-2,2560x1440@59.95,1600x0,1"
    ];

    workspace = [
      "1, monitor:DP-1, default:true"
      "2, monitor:DP-2, default:true"
      "3, monitor:HDMI-A-2, default:true"
      "4, monitor:DP-1"
      "5, monitor:DP-1"
    ];
  };
}
