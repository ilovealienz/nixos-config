{ pkgs, ... }:

{
  services.xserver.enable = true;
  services.displayManager.sddm.enable = true;
  services.desktopManager.plasma6.enable = true;

  environment.sessionVariables = {
    XDG_CURRENT_DESKTOP = "KDE";
    DESKTOP_SESSION = "plasma";
  };

  environment.plasma6.excludePackages = with pkgs.kdePackages; [
    elisa
    kate
  ];

  environment.systemPackages =
    (with pkgs.kdePackages; [
      kdeconnect-kde
    ])
    ++ (with pkgs; [
      rofi
      rofimoji
    ]);

  programs.ydotool.enable = true;
}
