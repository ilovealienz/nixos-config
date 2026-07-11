{ pkgs, ... }:

{
  services.xserver.enable = true;
  services.xserver.desktopManager.xfce.enable = true;
  services.displayManager.lightdm.enable = true;

  environment.xfce.excludePackages = with pkgs.xfce; [
    xfce4-screenshooter
    parole
    mousepad
  ];

  environment.systemPackages = with pkgs; [
    imv
  ];
}
