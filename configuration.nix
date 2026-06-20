{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./modules/core-packages.nix
    ./modules/gaming.nix
    ./modules/media.nix
    # ./modules/amd.nix
    ./modules/social.nix
    ./modules/dev.nix
    # ./hosts/pc-system.nix
    # ./modules/nvidia.nix
    # ./modules/intel.nix
  ];

  # Bootloader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Hostname
  networking.hostName = "nixos";
  networking.networkmanager.enable = true;

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
  xorg.libxcb
];

  # XDG portal
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
    config.common.default = "*";
  };

  services.flatpak.enable = true;

  # KDE Plasma
  services.xserver.enable = true;
  services.displayManager.sddm.enable = true;
  services.desktopManager.plasma6.enable = true;

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
  XDG_CURRENT_DESKTOP = "KDE";
  DESKTOP_SESSION = "plasma";
  };


  environment.shellAliases = {
  chromium-browser = "brave";
  x-www-browser = "brave";
  };

  # Remove unwanted KDE packages
  environment.plasma6.excludePackages = with pkgs.kdePackages; [
    elisa
    kate
  ];

  # User
  users.users.pc = {
    isNormalUser = true;
    description = "pc";
    extraGroups = [ "networkmanager" "wheel" ];
    shell = pkgs.zsh;
  };

  # Home Manager
  home-manager.users.pc = import ./home.nix;
  home-manager.backupFileExtension = "backup";

  # Unfree packages
  nixpkgs.config.allowUnfree = true;

  # Flakes
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # Weekly cleanup
  systemd.timers.nix-cleanup = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnCalendar = "weekly";
      Persistent = true;
    };
  };

  systemd.services.nix-cleanup = {
    serviceConfig.Type = "oneshot";
    script = ''
      nix-env --delete-generations +3 --profile /nix/var/nix/profiles/system
      nix-collect-garbage
      /run/current-system/bin/switch-to-configuration boot
    '';
  };

  system.stateVersion = "26.05";
}
