# Location for the weather and night light (Blackpool).
{ lib, ... }:
{
  options.my.location = {
    # per host: home-manager.users.pc.my.location.latitude
    latitude = lib.mkOption { type = lib.types.str; default = "53.8179442"; };
    longitude = lib.mkOption { type = lib.types.str; default = "-3.0509812"; };
  };
}
