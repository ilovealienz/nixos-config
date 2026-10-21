# Laptops: Intel graphics and the battery charge limit.
{ pkgs, ... }:
{
  # charge limit (toggled in the power menu): charge below 85%, stop at 90%
  services.udev.extraHwdb = ''
    battery:*:*:dmi:*
     CHARGE_LIMIT=85,90
  '';

  # Intel video acceleration: iHD (Broadwell and newer), i965 (older)
  hardware.graphics.extraPackages = [ pkgs.intel-media-driver pkgs.intel-vaapi-driver ];
}
