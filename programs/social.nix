{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    signal-desktop
    vesktop
    telegram-desktop
  ];
}
