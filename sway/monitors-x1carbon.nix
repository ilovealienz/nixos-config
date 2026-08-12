{ lib, osConfig, ... }:
let
  isX1 = osConfig.networking.hostName == "x1carbon";
in
lib.mkIf isX1 {
  wayland.windowManager.sway.config.output = {
    "eDP-1" = { mode = "2880x1800@60Hz"; position = "0 0"; scale = "1.5"; };
  };
}
