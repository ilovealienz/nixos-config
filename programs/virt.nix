{ pkgs, ... }:

{
  programs.virt-manager.enable = true;

  virtualisation.libvirtd = {
    enable = true;
    qemu.vhostUserPackages = with pkgs; [ virtiofsd ];
  };

  virtualisation.spiceUSBRedirection.enable = true;

  environment.systemPackages = with pkgs; [
    virtio-win
    dnsmasq
  ];

  users.groups.libvirtd.members = [ "pc" ];

  networking.firewall.trustedInterfaces = [ "virbr0" ];

  systemd.services.libvirtd-default-net = {
    after = [ "libvirtd.service" ];
    requires = [ "libvirtd.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig.Type = "oneshot";
    script = ''
      ${pkgs.libvirt}/bin/virsh net-autostart default || true
      ${pkgs.libvirt}/bin/virsh net-start default || true
    '';
  };
}
