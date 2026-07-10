{ pkgs, ... }:
{
  programs.hyprland = {
    enable = true;
    xwayland.enable = true;
  };

  services.displayManager.sddm.enable = true;
  services.displayManager.sddm.wayland.enable = true;

  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "Hyprland";
    DESKTOP_SESSION = "hyprland";
    NIXOS_OZONE_WL = "1";
  };

  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-hyprland ];

  environment.systemPackages = with pkgs; [
    waybar
    wofi
    hyprpaper
    hyprlock
    hypridle
  ];
}
