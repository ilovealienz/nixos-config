{ ... }:
{
  imports = [ ../hardware/intel-laptop.nix ];

  home-manager.users.pc.imports = [ ../sway/monitors/laptop.nix ];
}
