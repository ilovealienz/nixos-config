{ config, pkgs, lib, desktop, ... }:
{
  imports = [
    ./home/shell.nix
    ./home/autostart.nix
    ./home/packages.nix
    ./home/fastfetch.nix
    ./home/konsole.nix
    ./home/local-apps.nix
    ./home/stylix.nix
  ] ++ lib.optionals (desktop == "plasma") [
    ./home/plasma.nix
    ./home/rofi.nix
  ] ++ lib.optionals (desktop == "sway") [
    ./home/sway.nix
  ] ++ lib.optionals (desktop == "hyprland") [
    ./home/waybar.nix
    ./home/hyprland.nix
    ./home/hyprland-pc.nix
    ./home/fuzzel.nix
    ./home/mako.nix
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
