{ pkgs, ... }:
{
  programs.waybar = {
    enable = true;
    systemd.enable = false;
    settings.mainBar = {
      layer = "top";
      position = "top";
      height = 26;
      spacing = 0;

      modules-left = [ "hyprland/workspaces" ];
      modules-center = [ "hyprland/window" ];
      modules-right = [ "tray" "cpu" "memory" "pulseaudio" "network" "clock" ];

      "hyprland/workspaces" = {
        format = "{icon}";
        on-click = "activate";
        all-outputs = true;
        active-only = false;
        persistent-workspaces = { "*" = 5; };
        format-icons = {
         "1" = "󰈹"; # Firefox
    	 "2" = "󰭹"; # Spotify
    	 "3" = ""; # Play / MPV
    	 "4" = ""; # Terminal
    	 "5" = ""; # File manager
        };
      };

      "hyprland/window" = { max-length = 60; separate-outputs = true; };

      tray = { spacing = 10; icon-size = 16; };
      cpu = { format = "cpu {usage}%"; interval = 5; };
      memory = { format = "mem {percentage}%"; interval = 5; };
      pulseaudio = {
        format = "vol {volume}%";
        format-muted = "muted";
        on-click = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle";
        on-click-right = "pavucontrol";
      };
      network = {
        format-wifi = "{essid}";
        format-ethernet = "{ifname}";
        format-disconnected = "offline";
        on-click = "kitty --class kitty-float -e nmtui";
      };
      clock = {
        format = "{:%a %d %b  %H:%M}";
        format-alt = "{:(%a)%d/(%-m)%B/%Y %H:%M:%S}";
      };

      };
      style = ''
      * {
        font-family: "Inter", "DejaVu Sans", sans-serif;
        font-size: 13px;
        font-weight: 500;
        min-height: 0;
        border: none;
        border-radius: 0;
      }

      window#waybar {
        background: #24221c;
        color: #d4b07b;
      }

      #workspaces button {
        font-family: "MonaspiceAr Nerd Font", monospace;
        font-size: 15px;
        padding: 0 7px;
        background: #24221c;
        color: #87765d;                      /* empty: muted */
        border-bottom: 2px solid transparent;
      }
      #workspaces button.occupied,
      #workspaces button.persistent {
        color: #d4b07b;                      /* has windows: sand */
      }
      #workspaces button.empty {
        color: #87765d;
      }
      #workspaces button.active {
        color: #e5a440;                      /* active: amber */
        border-bottom: 2px solid #e5a440;
      }
      #workspaces button.urgent {
        background: #e56b55;
        color: #24221c;
      }

      #window { color: #87765d; }

      #tray, #cpu, #memory, #pulseaudio, #network, #clock {
        padding: 0 9px;
        color: #d4b07b;
      }
    '';


  };
}
