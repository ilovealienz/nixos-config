{ pkgs, lib, ... }:
{
  programs.sway = {
    enable = true;
    wrapperFeatures.gtk = true;
    extraPackages = [ pkgs.foot ];   # bootstrap terminal — remove once kitty is confirmed
  };
  programs.dconf.enable = true;
  services.displayManager.ly.enable = true;
  services.blueman.enable = true;
  security.polkit.enable = true;
  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "sway";
    DESKTOP_SESSION = "sway";
    NIXOS_OZONE_WL = "1";
  };
  xdg.portal = {
    enable = true;
    extraPortals = [
      pkgs.xdg-desktop-portal-hyprland
      pkgs.xdg-desktop-portal-gtk
    ];
    config.sway = {
      default = [ "gtk" ];
      "org.freedesktop.impl.portal.ScreenCast" = lib.mkForce [ "hyprland" ];
      "org.freedesktop.impl.portal.Screenshot" = lib.mkForce [ "hyprland" ];
    };
  };
  fonts.packages = [ pkgs.nerd-fonts.monaspace ];
  environment.systemPackages = with pkgs; [
    kitty
    waybar
    fuzzel
    mako
    polkit_gnome
    swaybg
    swayidle
    swaylock
    wl-clipboard
    grim
    slurp
    playerctl
    brightnessctl
    libnotify
  ];
}
