{ config, pkgs, lib, ... }:
{
  imports = [
    ./home/shell.nix
    ./home/packages.nix
    ./home/autostart.nix
    ./home/fastfetch.nix
    ./home/local-apps.nix
    ./home/mime.nix
    ./sway/home.nix
  ];

  home.username = "pc";
  home.homeDirectory = "/home/pc";
  home.stateVersion = "26.05";
  home.packages = [ pkgs.sshfs ];

  programs.git = {
    enable = true;
    settings = {
      user.name = "ilovealienz";
    };
  };

  home.activation.mpvConfig = lib.hm.dag.entryAfter ["writeBoundary"] ''
    if [ ! -d "$HOME/.config/mpv" ]; then
      ${pkgs.git}/bin/git clone https://github.com/ilovealienz/my-mpv-config "$HOME/.config/mpv"
    fi
  '';
}
