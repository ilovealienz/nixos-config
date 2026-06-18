{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    vesktop
    telegram-desktop
    element-desktop
  ];
}
