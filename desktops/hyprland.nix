{ pkgs, ... }:
{
  programs.hyprland = {
    enable = true;
    xwayland.enable = true;
  };
  programs.dconf.enable = true;

  #TTY login manager
  services.displayManager.ly.enable = true;
  services.blueman.enable = true;

  # Polkit
  security.polkit.enable = true;

  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "Hyprland";
    DESKTOP_SESSION = "hyprland";
    NIXOS_OZONE_WL = "1";
  };

  # XDPH
  xdg.portal.extraPortals = [
    pkgs.xdg-desktop-portal-hyprland
    pkgs.xdg-desktop-portal-gtk
  ];

  fonts.packages = with pkgs; [
    nerd-fonts.monaspace
    inter
    dejavu_fonts
    noto-fonts
    noto-fonts-color-emoji

  ];

  environment.systemPackages = with pkgs; [
    # core stack
    kitty
    waybar
    fuzzel
    mako
    polkit_gnome
    swaybg

    # hyprland-native tools
    hyprlock
    hypridle

    # utilities
    wl-clipboard
    grim
    slurp
    playerctl
    brightnessctl
    pavucontrol
    libnotify
    feh
    wmenu
    xarchiver
    thunar-archive-plugin
    zip
    unzip
    p7zip
    unrar

    # cursor + GTK plumbing
    bibata-cursors
    gsettings-desktop-schemas
    glib

    # GUI file manager
    thunar
    tumbler
  ];
}
