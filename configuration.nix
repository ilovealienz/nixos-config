{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./hardware/graphics.nix
    ./hardware/audio.nix
    ./hardware/bluetooth.nix
    ./hardware/power.nix
    ./hardware/memory.nix
    ./desktop/system.nix
    ./programs/core-packages.nix
    ./programs/gaming.nix
    ./programs/media.nix
    ./programs/social.nix
    ./programs/dev.nix
    ./programs/language.nix
    ./programs/nix-ld.nix
    ./root-theme.nix
    ./theme.nix
  ];

  # Debloat
  services.speechd.enable = false;
  documentation.nixos.enable = false;
  documentation.doc.enable = false;
  programs.command-not-found.enable = false;
  services.printing.enable = false;
  systemd.services.ModemManager.enable = false;

  # Bootloader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  # the last 10 NixOS generations in the boot menu, so the boot partition
  # can't fill up between cleanups (Windows' entry isn't affected)
  boot.loader.systemd-boot.configurationLimit = 10;

  # SFTP support
  services.gvfs.enable = true;

  # Networking
  networking.networkmanager.enable = true;

  # GitHub's host key (nxpush, nxpull), from GitHub's docs; fingerprint
  # SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU
  programs.ssh.knownHosts."github.com".publicKey =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";

  # Firewall
  networking.firewall = {
    allowedTCPPorts = [ 53317 25567 ];
    allowedUDPPorts = [ 53317 25567 ];
  };

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

  # Flatpak
  services.flatpak.enable = true;

  # Keyboard
  services.xserver.xkb = { layout = "gb"; variant = ""; };
  console.keyMap = "uk";

  # Electron Wayland
  environment.sessionVariables = {
    ELECTRON_OZONE_PLATFORM_HINT = "auto";
  };

  # in a terminal: the default browser, and Brave for what wants Chromium
  environment.shellAliases = {
    chromium-browser = "brave";
    x-www-browser = "librewolf";
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
  # identical files in /nix/store are stored once (hard links)
  nix.settings.auto-optimise-store = true;

  # firmware updates: fwupdmgr refresh && fwupdmgr update
  services.fwupd.enable = true;

  system.stateVersion = "26.05";
}
