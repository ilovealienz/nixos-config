# What the menus in the other sway files share: the fuzzel config the
# pickers use (emoji.nix, audio-menu.nix, window-picker.nix) and wmenu's
# colours from the theme (screenshot.nix, screenrec.nix, dnd.nix).
{ config, pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  # fuzzel 1.14 has no --override, so the pickers get their own config: your
  # fuzzel settings with a monospace font (so the columns line up) and
  # Shift+Enter moved from execute-input to custom-1.
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
