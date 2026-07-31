{
  description = "ilovealienz's NixOS config";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    home-manager = {
      url = "github:nix-community/home-manager/release-26.05";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, home-manager, ... }:
  let
    system = "x86_64-linux";

    sharedModules = [
      home-manager.nixosModules.home-manager
      { home-manager.useGlobalPkgs = true; }
    ];

    mkHost = { hostname, extraModules ? [] }:
      nixpkgs.lib.nixosSystem {
        inherit system;
        modules = sharedModules ++ extraModules ++ [
          ./configuration.nix
          { networking.hostName = hostname; }
        ];
      };
  in {
    nixosConfigurations = {
      pc = mkHost {
        hostname = "pc";
        extraModules = [ ./hosts/pc.nix ];
      };

      laptop = mkHost {
        hostname = "laptop";
        extraModules = [ ./hosts/laptop.nix ];
      };
    };
  };
}
