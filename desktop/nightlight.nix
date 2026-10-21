# Warmer screen after sunset (wlsunset).
{ config, lib, ... }:
{
  services.wlsunset = {
    enable = true;
    inherit (config.my.location) latitude longitude;
  };

  systemd.user.services.wlsunset = {
    Unit = {
      After = lib.mkForce [ "sway-session.target" ];
      PartOf = lib.mkForce [ "sway-session.target" ];
    };
    Install.WantedBy = lib.mkForce [ "sway-session.target" ];
  };
}
