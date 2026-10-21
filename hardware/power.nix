{ ... }:
{
  # power menu backends: profiles and the battery charge limit
  services.power-profiles-daemon.enable = true;
  services.upower.enable = true;
  # charge limit on by default (first boot only; the menu's choice sticks)
  systemd.tmpfiles.rules = [
    "f /var/lib/upower/charging-threshold-status 0640 root root - 1"
  ];
}
