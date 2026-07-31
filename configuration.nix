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

  # Bootloader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

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
  time.timeZone = "Europe/London";
  i18n.defaultLocale = "en_GB.UTF-8";
  i18n.extraLocaleSettings = {
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

  # Printing
  services.printing.enable = true;

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
