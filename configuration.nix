{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./hyprland/system.nix
    ./programs/core-packages.nix
    ./programs/gaming.nix
    ./programs/media.nix
    ./programs/social.nix
    ./programs/dev.nix
  ];

  # Debloat
  services.speechd.enable = false;
  documentation.nixos.enable = false;
  documentation.doc.enable = false;
  programs.command-not-found.enable = false;
  services.printing.enable = false;
  systemd.services.ModemManager.enable = false;

  # ROOT THEME
  environment.systemPackages = with pkgs; [ adw-gtk3 ];

  environment.etc."root-gtk/settings.ini".text = ''
    [Settings]
    gtk-theme-name=adw-gtk3-dark
    gtk-icon-theme-name=Gruvbox-Plus-Dark
    gtk-cursor-theme-name=Bibata-Modern-Classic
    gtk-application-prefer-dark-theme=1
  '';

  environment.etc."root-gtk/gtk.css".text = ''
    @define-color window_bg_color #2a1a18;
    @define-color window_fg_color #e8c9a0;
    @define-color view_bg_color #331f1c;
    @define-color view_fg_color #e8c9a0;
    @define-color headerbar_bg_color #3a1f1c;
    @define-color headerbar_fg_color #e8c9a0;
    @define-color popover_bg_color #331f1c;
    @define-color popover_fg_color #e8c9a0;
    @define-color card_bg_color #331f1c;
    @define-color dialog_bg_color #2a1a18;
    @define-color dialog_fg_color #e8c9a0;
    @define-color sidebar_bg_color #331f1c;
    @define-color sidebar_fg_color #e8c9a0;
    @define-color accent_color #e56b55;
    @define-color accent_bg_color #e56b55;
    @define-color accent_fg_color #2a1a18;
    @define-color theme_bg_color #2a1a18;
    @define-color theme_fg_color #e8c9a0;
    @define-color theme_base_color #331f1c;
    @define-color theme_text_color #e8c9a0;
    @define-color theme_selected_bg_color #e56b55;
    @define-color theme_selected_fg_color #2a1a18;
    @define-color borders #5a2f2a;
  '';

  systemd.tmpfiles.rules = [
    "d /root/.config 0700 root root -"
    "d /root/.config/gtk-3.0 0700 root root -"
    "d /root/.config/gtk-4.0 0700 root root -"
    "L+ /root/.config/gtk-3.0/settings.ini - - - - /etc/root-gtk/settings.ini"
    "L+ /root/.config/gtk-3.0/gtk.css      - - - - /etc/root-gtk/gtk.css"
    "L+ /root/.config/gtk-4.0/settings.ini - - - - /etc/root-gtk/settings.ini"
    "L+ /root/.config/gtk-4.0/gtk.css      - - - - /etc/root-gtk/gtk.css"
  ];


  # Bootloader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # SFTP support
  services.gvfs.enable = true;

  # Networking
  networking.networkmanager.enable = true;

  # Bluetooth
  hardware.bluetooth.enable = true;
  hardware.bluetooth.powerOnBoot = true;
  hardware.bluetooth.settings = {
    General = {
      Experimental = true;   # enables battery % reporting for supported devices
    };
  };

  # Timezone & locale
  # Timezone & locale
  time.timeZone = "Europe/London";

  i18n = {
    defaultLocale = "en_GB.UTF-8";

    supportedLocales = [
      "C.UTF-8/UTF-8"
      "en_GB.UTF-8/UTF-8"
      "en_US.UTF-8/UTF-8"
    ];

    extraLocaleSettings = {
      LC_ADDRESS = "en_GB.UTF-8";
      LC_IDENTIFICATION = "en_GB.UTF-8";
      LC_MEASUREMENT = "en_GB.UTF-8";
      LC_MONETARY = "en_GB.UTF-8";
      LC_NAME = "en_GB.UTF-8";
      LC_NUMERIC = "en_GB.UTF-8";
      LC_PAPER = "en_GB.UTF-8";
      LC_TELEPHONE = "en_GB.UTF-8";
      LC_TIME = "en_GB.UTF-8";
    };
  };

  # ZSH
  programs.zsh.enable = true;

  # nix-ld for generic Linux binaries
  programs.nix-ld.enable = true;
  programs.nix-ld.libraries = with pkgs; [
    wayland
    libxkbcommon
    libGL
    mesa
    vulkan-loader
    libx11
    libxcursor
    libxrandr
    libxi
    fontconfig
    freetype
    openssl
    gtk3
    pango
    harfbuzz
    atk
    cairo
    gdk-pixbuf
    glib
    zlib
    stdenv.cc.cc.lib
    libxcb
    libpulseaudio
    alsa-lib
    pipewire
    libjack2
  ];

  # XDG portal
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
    config.common.default = "*";
  };

  # Flatpak
  services.flatpak.enable = true;

  # Keyboard
  services.xserver.xkb = { layout = "gb"; variant = ""; };
  console.keyMap = "uk";

  # Audio
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };

  # Electron Wayland
  environment.sessionVariables = {
    ELECTRON_OZONE_PLATFORM_HINT = "auto";
  };

  environment.shellAliases = {
    chromium-browser = "brave";
    x-www-browser = "brave";
  };

  # User
  users.users.pc = {
    isNormalUser = true;
    description = "pc";
    extraGroups = [ "networkmanager" "wheel" "ydotool" ];
    shell = pkgs.zsh;
  };

  # Home Manager
  home-manager.users.pc = import ./home.nix;
  home-manager.backupFileExtension = "backup";

  # Unfree packages
  nixpkgs.config.allowUnfree = true;

  # Flakes
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  system.stateVersion = "26.05";
}
