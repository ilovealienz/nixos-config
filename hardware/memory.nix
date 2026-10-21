{ ... }:
{
  # compressed swap in RAM
  zramSwap.enable = true;

  # stops the biggest program when memory and swap are both under 10%.
  # Names match the first 15 characters of the process name.
  services.earlyoom = {
    enable = true;
    enableNotifications = true;
    extraArgs = [
      # browsers (LibreWolf tabs are "Isolated Web Content"), Minecraft
      "--prefer" "^(librewolf|Isolated.Web.Co|Web.Content|brave|java)$"
      "--avoid" "^(sway|Xwayland|waybar|kitty|foot)$"    # never the session
    ];
  };
}
