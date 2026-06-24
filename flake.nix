{
  description = "ilovealienz's NixOS config";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";

    home-manager = {
      url = "github:nix-community/home-manager/release-26.05";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    plasma-manager = {
      url = "github:nix-community/plasma-manager";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.home-manager.follows = "home-manager";
    };
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, home-manager, plasma-manager, ... }:
  let
    system = "x86_64-linux";
    pkgs-unstable = nixpkgs-unstable.legacyPackages.${system};
    sharedModules = [
      home-manager.nixosModules.home-manager
      {
        home-manager.sharedModules = [
          plasma-manager.homeModules.plasma-manager
        ];
        home-manager.useGlobalPkgs = true;
      }
    ];
  in {
    nixosConfigurations = {
      pc = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit pkgs-unstable; };
        modules = sharedModules ++ [
          ./configuration.nix
          ./hosts/pc-system.nix
          ./modules/amd.nix
          { networking.hostName = "pc"; }
        ];
      };

      generic = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit pkgs-unstable; };
        modules = sharedModules ++ [
          ./configuration.nix
          { networking.hostName = "generic"; }
        ];
      };
    };
  };
}
