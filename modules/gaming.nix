{ pkgs, ... }:

{
  programs.steam.enable = true;
  programs.gamemode.enable = true;

  environment.systemPackages = with pkgs; [
    lutris
    bottles
    wine
    winetricks
    prismlauncher
  ];
}
