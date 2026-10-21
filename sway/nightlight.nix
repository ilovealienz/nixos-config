# Warmer screen colours after sunset (wlsunset), for Blackpool. Runs with
# the sway session.
{ lib, ... }:
{
  services.wlsunset = {
    enable = true;
    latitude = "53.8";
    longitude = "-3.0";
  };

  systemd.user.services.wlsunset = {
    Unit = {
      After = lib.mkForce [ "sway-session.target" ];
      PartOf = lib.mkForce [ "sway-session.target" ];
    };
    Install.WantedBy = lib.mkForce [ "sway-session.target" ];
  };
}
