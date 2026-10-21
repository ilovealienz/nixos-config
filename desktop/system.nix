{ pkgs, ... }:
{
  programs.sway = {
    enable = true;
    wrapperFeatures.gtk = true;
    extraPackages = [ ];        # no default foot/dmenu
  };
  programs.dconf.enable = true;

  services.displayManager.ly.enable = true;
  services.blueman.enable = true;
  security.polkit.enable = true;

  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "sway";
    DESKTOP_SESSION = "sway";
    NIXOS_OZONE_WL = "1";
    TERMINAL = "kitty";
  };

  # portals: gtk for most things, wlr for screen sharing and screenshots
  xdg.portal = {
    enable = true;
    # links from sandboxed apps (Steam) open in the normal browser
    xdgOpenUsePortal = true;
    config.common.default = "*";
    wlr = {
      enable = true;
      settings.screencast = {
        chooser_type = "dmenu";
        chooser_cmd = "${pkgs.callPackage ../pkgs/share-chooser.nix { }}/bin/share-chooser";
      };
    };
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
    config.sway = {
      default = [ "gtk" ];
      "org.freedesktop.impl.portal.ScreenCast" = [ "wlr" ];
      "org.freedesktop.impl.portal.Screenshot" = [ "wlr" ];
    };
  };

  fonts.packages = with pkgs; [
    nerd-fonts.monaspace
    inter
    dejavu_fonts
    noto-fonts
    noto-fonts-color-emoji
  ];

  # icons and cursor are here too for root's apps (root-theme.nix)
  environment.systemPackages = with pkgs; [
    # core stack
    wmenu
    polkit_gnome

    # utilities
    wl-clipboard
    grim
    slurp
    playerctl
    brightnessctl
    pavucontrol
    libnotify
    imv
    xarchiver
    thunar-archive-plugin
    zip
    unzip
    p7zip
    unrar

    # cursor + icons + gtk plumbing
    bibata-cursors
    gruvbox-plus-icons
    gsettings-desktop-schemas
    glib

    # file manager
    thunar
    tumbler
  ];
}
