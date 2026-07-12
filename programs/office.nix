{ pkgs, desktop, ... }:

{
  environment.systemPackages = with pkgs; [
    (if desktop == "plasma" then libreoffice-qt else libreoffice)
  ];
}
