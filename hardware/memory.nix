{ ... }:
{
  # compressed swap in RAM
  zramSwap.enable = true;

  # when memory and swap both run low (under 10% each), the program using
  # the most memory is stopped before the whole desktop freezes; a
  # notification says which. --prefer and --avoid match process names
  # (their first 15 characters)
  services.earlyoom = {
    enable = true;
    enableNotifications = true;
    extraArgs = [
      # browsers (LibreWolf's tabs are "Isolated Web Content"), Minecraft
      "--prefer" "^(librewolf|Isolated.Web.Co|Web.Content|brave|java)$"
      "--avoid" "^(sway|Xwayland|waybar|kitty|foot)$"    # last: the session itself
    ];
  };
}
