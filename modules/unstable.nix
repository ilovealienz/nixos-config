{ pkgs-unstable, ... }:

{
  environment.systemPackages = [
    pkgs-unstable.yacreader
  ];
}
