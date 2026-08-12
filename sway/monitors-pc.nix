{ lib, osConfig, ... }:
let
  isPc = osConfig.networking.hostName == "pc";
in
lib.mkIf isPc {
  wayland.windowManager.sway = {
    config = {
      output = {
        # 4K at 1440p on top, two 1080p below. DP-1 is main/centre.
        "DP-1"     = { mode = "1920x1080@143.86Hz"; position = "1920 1440"; scale = "1"; };
        "DP-2"     = { mode = "1920x1080@165Hz";    position = "0 1440";    scale = "1"; };
        "HDMI-A-2" = { mode = "2560x1440@59.95Hz";  position = "1600 0";    scale = "1"; };
      };
      workspaceOutputAssign = [
        { workspace = "1"; output = "DP-1"; }
        { workspace = "2"; output = "DP-2"; }
        { workspace = "3"; output = "HDMI-A-2"; }
      ];
    };

    extraConfig = ''
      # each monitor starts on its own workspace
      exec swaymsg 'focus output HDMI-A-2; workspace 3; focus output DP-2; workspace 2; focus output DP-1; workspace 1'
    '';
  };
}
