# Shared by the menus: the pickers' fuzzel config and wmenu's colours.
{ config, pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  # your fuzzel config, monospace, Shift+Enter as custom-1 (fuzzel has no
  # --override)
  pickerFuzzel = pkgs.writeText "window-picker-fuzzel.ini"
    (lib.generators.toINI { } (config.programs.fuzzel.settings // {
      main = config.programs.fuzzel.settings.main // {
        font = "MonaspiceAr Nerd Font:size=11";
      };
      key-bindings = {
        execute-input = "Control+Shift+Return";
        custom-1 = "Shift+Return";
      };
    }));
in
{
  options.my = {
    pickerFuzzel = lib.mkOption {
      type = lib.types.package;
      readOnly = true;
      description = "fuzzel config for the pickers: monospace, Shift+Enter is custom-1";
    };
    wmenuColours = lib.mkOption {
      type = lib.types.str;
      readOnly = true;
      description = "wmenu's colour options, from the theme";
    };
  };

  config.my = {
    inherit pickerFuzzel;
    wmenuColours = "-N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.onAccent} -M ${c.accent} -m ${c.onAccent}";
  };
}
