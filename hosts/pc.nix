{ ... }:

{
  imports = [ ../programs/virt.nix ];

  home-manager.users.pc.imports = [ ../desktop/monitors/pc.nix ];

  # streaming games from here (Sunshine, programs/gaming.nix)
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
