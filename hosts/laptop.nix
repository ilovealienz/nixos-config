{ pkgs, ... }:

{
  imports = [ ../hardware/graphics.nix ];

  # VAAPI video encode/decode on Intel graphics; mesa only covers AMD.
  # iHD is for Broadwell and newer, i965 for older chips.
  hardware.graphics.extraPackages = [ pkgs.intel-media-driver pkgs.intel-vaapi-driver ];
}
