{ pkgs, ... }:

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
  # keep the last 10 generations in the boot menu
  boot.loader.systemd-boot.configurationLimit = 10;
  # no kernel command-line editing at boot (init=/bin/sh is a root shell)
  boot.loader.systemd-boot.editor = false;

  # SFTP support
  services.gvfs.enable = true;

  # Networking
  networking.networkmanager.enable = true;

  # GitHub's SSH host key (nxpush, nxpull)
  programs.ssh.knownHosts."github.com".publicKey =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";

  # Firewall
  networking.firewall = {
    # LocalSend (53317), Minecraft (25567)
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

  # shell aliases for generic browser names
  environment.shellAliases = {
    chromium-browser = "brave";
    x-www-browser = "librewolf";
  };

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
  # an old .backup no longer blocks activation
  home-manager.overwriteBackup = true;

  # Unfree packages
  nixpkgs.config.allowUnfree = true;

  # Flakes
  nix.settings = {
    experimental-features = [ "nix-command" "flakes" ];
    # free space mid-build when under 1 GiB, up to 5 GiB
    min-free = 1024 * 1024 * 1024;
    max-free = 5 * 1024 * 1024 * 1024;
    # no "download buffer is full" stalls on big downloads
    download-buffer-size = 256 * 1024 * 1024;
    # give up on an unreachable cache after 5s, not 5 minutes
    connect-timeout = 5;
  };
  # builds yield to the desktop
  nix.daemonCPUSchedPolicy = "batch";
  nix.daemonIOSchedClass = "idle";
  # flakes only: no stale channel nixpkgs
  nix.channel.enable = false;
  # hard-link identical store files, weekly (on every build it slows builds)
  nix.optimise = {
    automatic = true;
    dates = [ "weekly" ];
  };

  # weekly cleanup, keeping the last 3 generations (nxclean runs it now)
  programs.nh = {
    enable = true;
    clean = {
      enable = true;
      dates = "weekly";
      extraArgs = "--keep 3";
    };
  };

  # firmware updates: fwupdmgr refresh && fwupdmgr update
  services.fwupd.enable = true;

  # disk health warnings, as notifications
  services.smartd = {
    enable = true;
    notifications.systembus-notify.enable = true;
  };

  system.stateVersion = "26.05";
}
