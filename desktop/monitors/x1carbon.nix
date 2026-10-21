# imported by hosts/x1carbon.nix
{ ... }:
{
  wayland.windowManager.sway.config.output = {
    "eDP-1" = { mode = "2880x1800@60Hz"; position = "0 0"; scale = "2.0"; };
  };
}
