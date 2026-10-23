{ pkgs, ... }:

{
  imports = [ ../programs/virt.nix ];

  home-manager.users.pc.imports = [ ../desktop/monitors/pc.nix ];

  # a second sway in ly without direct scanout, for gamescope in fullscreen:
  # sway switching scanout on and off as the cursor moves corrupts the
  # screen on this card (sway#8929)
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
