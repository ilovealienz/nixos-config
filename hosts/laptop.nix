{ pkgs, ... }:

{
  imports = [ ../hardware/graphics.nix ];

  # battery charge limit (on/off in the power menu): start charging below
  # 85%, stop at 90%. Mostly unplugged, so close to a full charge.
  services.udev.extraHwdb = ''
    battery:*:*:dmi:*
     CHARGE_LIMIT=85,90
  '';

  # VAAPI video encode/decode on Intel graphics; mesa only covers AMD.
  # iHD is for Broadwell and newer, i965 for older chips.
  hardware.graphics.extraPackages = [ pkgs.intel-media-driver pkgs.intel-vaapi-driver ];
}
