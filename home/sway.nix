{ pkgs, lib, osConfig, ... }:
let
  host = osConfig.networking.hostName;
in
{
  wayland.windowManager.sway = {
    enable = true;
    config = {
      modifier = "Mod4";          # Super
      terminal = "kitty";
      menu = "fuzzel";

      input."type:keyboard".xkb_layout = "gb";

      bars = [{ command = "${pkgs.waybar}/bin/waybar"; }];

      startup = [
        { command = "${pkgs.polkit_gnome}/libexec/polkit-gnome-authentication-agent-1"; }
      ];

      output = {
        "*".bg = "#2e3440 solid_color";
      }
      # ── PC: 4K@1440p above, two 1080p below (DP-1 main/centre) ──
      // lib.optionalAttrs (host == "pc") {
        "DP-1"     = { mode = "1920x1080@143.855Hz"; position = "1920 1440"; };   # MSI G24C6, 144Hz, MAIN
        "DP-2"     = { mode = "1920x1080@165.001Hz"; position = "0 1440"; };      # MSI G2422C, left
        "HDMI-A-2" = { mode = "2560x1440@59.951Hz"; scale = "1"; position = "1600 0"; };  # LG 4K → 1440p, above
      }
      # ── Laptop: fill in real names from `swaymsg -t get_outputs` on that machine ──
      // lib.optionalAttrs (host == "laptop") {
        "eDP-1" = { mode = "1920x1080@60Hz"; position = "0 0"; };
      };
    };
  };

  services.mako.enable = true;

  services.swayidle = {
    enable = true;
    timeouts = [
      { timeout = 600; command = "${pkgs.swaylock}/bin/swaylock -f -c 2e3440"; }
    ];
  };
}
