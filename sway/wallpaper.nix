# The wallpaper: walls/1.png, or the one picked in theme-edit, which copies
# it to ~/.config/theme/wallpaper. That one is set by wallpaper-set at
# startup and on reload, not by an output line in the config: sway checks
# the config while building, and the build can't see files in your home.
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;
  wallMode = osConfig.theme.wallpaperMode;
  wallpaperSet = pkgs.writeShellApplication {
    name = "wallpaper-set";
    runtimeInputs = [ pkgs.sway ];
    text = ''
      wall="''${XDG_CONFIG_HOME:-$HOME/.config}/theme/wallpaper"
      if [ -f "$wall" ]; then
        swaymsg -q "output * bg \"$wall\" ${toString wallMode} #${c.bg}"
      else
        swaymsg -q "output * bg ${../walls/1.png} fill"
      fi
    '';
  };
in
{
  # no wallpaper picked: walls/1.png, by an output line (sway runs swaybg)
  wayland.windowManager.sway.config.output = lib.mkIf (wallMode == null) { "*".bg = "${../walls/1.png} fill"; };

  wayland.windowManager.sway.config.startup =
    lib.optional (wallMode != null) { command = lib.getExe wallpaperSet; always = true; };
}
