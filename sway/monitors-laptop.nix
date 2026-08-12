{ lib, osConfig, ... }:
let
  isLaptop = osConfig.networking.hostName == "laptop";
in
lib.mkIf isLaptop {
  wayland.windowManager.sway.config.output = {
    "eDP-1" = { mode = "1920x1080@60Hz"; position = "0 0"; scale = "1.25"; };
  };
}
