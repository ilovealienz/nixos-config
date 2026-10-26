{ config, pkgs, lib, ... }:

{
  imports = [ ../programs/virt.nix ];

  home-manager.users.pc.imports = [ ../desktop/monitors/pc.nix ];

  # second boot entry on the newest kernel
  specialisation.latest-kernel.configuration = {
    boot.kernelPackages = lib.mkForce pkgs.linuxPackages_latest;
    # lets nh rebuild into it
    environment.etc."specialisation".text = "latest-kernel";
  };

  # rebuilding from a specialisation leaves the boot default on an older generation
  boot.loader.systemd-boot.extraInstallCommands = ''
    esp=${config.boot.loader.efi.efiSysMountPoint}
    newest=$(${pkgs.coreutils}/bin/ls "$esp/loader/entries" | ${pkgs.gnused}/bin/sed -n 's/^nixos-generation-\([0-9]*\)\.conf$/\1/p' | ${pkgs.coreutils}/bin/sort -n | ${pkgs.coreutils}/bin/tail -n 1)
    if [ -n "$newest" ]; then
      ${pkgs.gnused}/bin/sed -i "s/^default .*/default nixos-generation-$newest.conf/" "$esp/loader/loader.conf"
    fi
  '';

  # sway without direct scanout, for fullscreen gamescope (sway#8929)
  services.displayManager.sessionPackages = [
    ((pkgs.writeTextDir "share/wayland-sessions/sway-noscanout.desktop" ''
      [Desktop Entry]
      Name=Sway (no direct scanout)
      Comment=for gamescope in fullscreen
      Exec=env WLR_SCENE_DISABLE_DIRECT_SCANOUT=1 sway
      Type=Application
      DesktopNames=sway;wlroots
    '').overrideAttrs { passthru.providedSessions = [ "sway-noscanout" ]; })
  ];

  # Sunshine game streaming
  services.sunshine.openFirewall = true;

  fileSystems."/mnt/Shared" = {
    device = "/dev/disk/by-label/Shared";
    fsType = "auto";
    options = [ "nosuid" "nodev" "nofail" "x-gvfs-show" ];
  };

  fileSystems."/mnt/NVME2" = {
    device = "/dev/disk/by-label/NVME2";
    fsType = "auto";
    options = [ "nosuid" "nodev" "nofail" "x-gvfs-show" ];
  };

  fileSystems."/mnt/NVME3" = {
    device = "/dev/disk/by-label/NVME3";
    fsType = "auto";
    options = [ "nosuid" "nodev" "nofail" "x-gvfs-show" ];
  };
}
