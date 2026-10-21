# Super+Shift+Tab: every window in fuzzel, grouped by workspace. Enter goes
# to it (or to the workspace, on a header), Shift+Enter opens an action menu
# (bring here, float, scratchpad, close, force kill).
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
