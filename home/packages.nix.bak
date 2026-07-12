{ pkgs, ... }:

{
  home.packages = with pkgs; [
    papirus-icon-theme
  ];

  programs.nh = {
  enable = true;
  clean = {
    enable = true;
    dates = "weekly";
    extraArgs = "--keep 3";
   };
 };
}










