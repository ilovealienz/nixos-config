{ ... }:
{
  # power menu (sway/power.nix): power-profiles-daemon for performance /
  # balanced / power-saver, upower for the battery charge limit (and to
  # tell power-profiles-daemon when a laptop is on battery)
  services.power-profiles-daemon.enable = true;
  services.upower.enable = true;
  # battery charge limit on by default, on laptops that support one. This
  # only creates the setting the first time; after that upower keeps
  # whatever the power menu last set. Limits: upower's 75-80% unless a
  # host sets CHARGE_LIMIT (hardware/intel-laptop.nix).
  systemd.tmpfiles.rules = [
    "f /var/lib/upower/charging-threshold-status 0640 root root - 1"
  ];
}
