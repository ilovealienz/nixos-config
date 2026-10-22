# The nx commands: one script each in ../scripts/nx/, checked with
# shellcheck when built. Listed: the programs each needs beyond the system's
# (sudo, git, nix, nh, iptables...). nxedit is Rust: ../pkgs/nxedit/
{ pkgs, lib, osConfig, ... }:
let
  nx = name: deps: pkgs.writeShellApplication {
    inherit name;
    runtimeInputs = deps;
    text = builtins.readFile ../scripts/nx/${name}.sh;
  };

  nxeditBin = pkgs.rustPlatform.buildRustPackage {
    pname = "nxedit";
    version = "1.0.0";
    src = ../pkgs/nxedit;
    cargoLock.lockFile = ../pkgs/nxedit/Cargo.lock;
  };
  # the theme's colours and bat for the preview (a theme change only
  # rebuilds this wrapper)
  nxedit = pkgs.runCommand "nxedit" {
    nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
    meta.mainProgram = "nxedit";
  } ''
    makeWrapper ${nxeditBin}/bin/nxedit $out/bin/nxedit \
      --add-flags "--colors ${pkgs.writeText "nxedit-colors.json" (builtins.toJSON osConfig.theme.colors)}" \
      --prefix PATH : ${lib.makeBinPath [ pkgs.bat ]}
  '';
in
{
  home.packages = with pkgs; [
    (nx "nxrebuild" [ diffutils ])
    (nx "nxupdate" [ procps ])
    (nx "nxpull" [ ])
    (nx "nxpush" [ ])
    (nx "nxcheck" [ coreutils gnused gnugrep ])
    nxedit
    (nx "nxsearch" [ nix-search-tv fzf gnused ])
    (nx "nxsrun" [ ])
    (nx "nxrun" [ ])
    (nx "nxport" [ ])
    (nx "nxaudio" [ alsa-utils ])
    (nx "nxclean" [ ])
  ];
}
