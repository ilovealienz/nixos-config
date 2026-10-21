# Where you are: the weather (weather.nix) and when the screen warms up
# (nightlight.nix). Blackpool.
{ lib, ... }:
{
  options.my.location = {
    # a host file can set its own: home-manager.users.pc.my.location.latitude
    latitude = lib.mkOption { type = lib.types.str; default = "53.8179442"; };
    longitude = lib.mkOption { type = lib.types.str; default = "-3.0509812"; };
  };
}
