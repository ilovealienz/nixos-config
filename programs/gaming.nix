{ pkgs, ... }:

{
  programs.steam.enable = true;
  programs.gamemode.enable = true;

  environment.systemPackages = with pkgs; [
    bottles
    (prismlauncher.override { jdks = [ jdk21 jdk17 ]; })
    protonplus
  ];
}
