{ pkgs, ... }:

{
  programs.steam.enable = true;
  programs.gamemode.enable = true;

  services.sunshine = {
    enable = true;
    autoStart = false;
    capSysAdmin = true;
    openFirewall = true;
  };

  services.udev.extraRules = ''
    # GameCube Controller Adapter
    SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0337", TAG+="uaccess"

    # Wiimotes / DolphinBar
    SUBSYSTEM=="hidraw*", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0306", TAG+="uaccess"
    SUBSYSTEM=="hidraw*", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0330", TAG+="uaccess"
  '';

  environment.systemPackages = with pkgs; [
    (prismlauncher.override { jdks = [ jdk25 ]; })
    protonup-qt
    dolphin-emu
  ];
}
