# Wallpaper: walls/1.png, or theme-edit's pick (~/.config/theme/wallpaper),
# set by wallpaper-set at startup since the build can't see your home.
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
  # the default wallpaper
  wayland.windowManager.sway.config.output = lib.mkIf (wallMode == null) { "*".bg = "${../walls/1.png} fill"; };

  wayland.windowManager.sway.config.startup =
    lib.optional (wallMode != null) { command = lib.getExe wallpaperSet; always = true; };
}
