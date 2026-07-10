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
    pkgs-unstable = import nixpkgs-unstable {
      inherit system;
      config.allowUnfree = true;
    };
    sharedModules = [
      home-manager.nixosModules.home-manager
      {
        home-manager.sharedModules = [
          plasma-manager.homeModules.plasma-manager
        ];
        home-manager.useGlobalPkgs = true;
      }
    ];

    mkHost = { hostname, desktop, extraModules ? [] }:
      nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit pkgs-unstable desktop; };
        modules = sharedModules ++ extraModules ++ [
          ./configuration.nix
          { networking.hostName = hostname; }
        ];
      };
  in {
    nixosConfigurations = {
      pc = mkHost {
        hostname = "pc";
        desktop = "plasma";
        extraModules = [ ./hosts/pc.nix ];
      };

      laptop = mkHost {
        hostname = "laptop";
        desktop = "plasma";
        extraModules = [ ./hosts/laptop.nix ];
      };

      generic = mkHost {
        hostname = "generic";
        desktop = "plasma";
      };
    };
  };
}
