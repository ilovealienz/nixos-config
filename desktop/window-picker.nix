# Super+Shift+Tab: window picker, grouped by workspace. Shift+Enter for
# actions (bring here, float, scratchpad, close, kill).
# Source: ../pkgs/window-picker/main.go
{ config, pkgs, lib, ... }:
let
  windowPicker = pkgs.buildGoModule {
    pname = "window-picker";
    version = "2.6";
    src = ../pkgs/window-picker;
    vendorHash = null;   # standard library only
    ldflags = [
      "-s" "-w"
      "-X main.fuzzel=${pkgs.fuzzel}/bin/fuzzel"
      "-X main.config=${config.my.pickerFuzzel}"
    ];
    meta.mainProgram = "window-picker";
  };
in
{
  wayland.windowManager.sway.config.keybindings."Mod4+Shift+Tab" = "exec ${lib.getExe windowPicker}";
}
