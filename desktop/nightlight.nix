# Warmer screen colours after sunset (wlsunset), where location.nix says
# you are. Runs with the sway session.
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
