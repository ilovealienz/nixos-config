{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    git
    curl
    wget
    htop
    unzip
    fastfetch
    brave
    vscodium
    vim
    mission-center
    librewolf
    gnome-disk-utility
  ];
}
