{ pkgs, ... }:
{
  programs.hyprland = {
    enable = true;
    xwayland.enable = true;
  };

  programs.dconf.enable = true;
  services.xserver.enable = true;
  services.displayManager.sddm.enable = true;

  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "Hyprland";
    DESKTOP_SESSION = "hyprland";
    NIXOS_OZONE_WL = "1";
  };

  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-hyprland pkgs.xdg-desktop-portal-gtk ];

  environment.systemPackages = with pkgs; [
    uwsm
    kitty
    waybar
    wofi
    hyprpaper
    hyprlock
    hypridle
    playerctl
    nwg-displays
    bibata-cursors
    xfce.thunar
    tumbler
    gsettings-desktop-schemas
    glib
    grim
    slurp
  ];
}
