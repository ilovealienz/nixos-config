{ lib, osConfig, ... }:
let
  isLaptop = osConfig.networking.hostName == "laptop";
in
lib.mkIf isLaptop {
  wayland.windowManager.hyprland.settings = {
    monitor = [
      "eDP-1,preferred,auto,1.25"
    ];
  };
}
