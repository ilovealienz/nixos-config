{ ... }:
{
  imports = [ ../hardware/intel-laptop.nix ];

  home-manager.users.pc.imports = [ ../desktop/monitors/x1carbon.nix ];
}
