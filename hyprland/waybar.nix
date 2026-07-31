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
      modules-right = [ "tray" "cpu" "memory" "pulseaudio" "battery" "network" "clock" "idle_inhibitor" ];

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
        on-click-right = "blueman-manager";
      };
      battery = {
        states = { warning = 30; critical = 15; };
        format = "bat {capacity}%";
        format-charging = "chg {capacity}%";
        format-plugged = "plug {capacity}%";
        tooltip-format = "{timeTo}";
      };
      clock = {
        format = "{:%a %d %b  %H:%M}";
        tooltip-format = "<tt>{calendar}</tt>";
        calendar = {
          mode = "month";
          weeks-pos = "right";
          format = {
            months    = "<span color='#e5a440'><b>{}</b></span>";
            days      = "<span color='#d4b07b'>{}</span>";
            weeks     = "<span color='#87765d'>W{}</span>";
            weekdays  = "<span color='#e18245'><b>{}</b></span>";
            today     = "<span color='#e56b55'><b><u>{}</u></b></span>";
          };
        };
        actions = {
          on-click-right  = "mode";
          on-scroll-up    = "shift_up";
          on-scroll-down  = "shift_down";
        };
      };
      idle_inhibitor = {
        format = "{icon}";
        format-icons = {
          activated = " ";
          deactivated = " ";
        };
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

      tooltip {
        background-color: #24221c;
        border: 0px solid #e5a440;
      }
      tooltip label {
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

      #tray, #cpu, #memory, #pulseaudio, #network, #battery, #clock {
        padding: 0 9px;
        color: #d4b07b;
      }

      /* thin toggle sliver — outline = idle on, filled = idle paused */
      #idle_inhibitor {
        min-width: 3px;
        margin: 0 0 0 4px;
        padding: 0;
        background: #473f31;
      }
      #idle_inhibitor.activated {
        background: #e5a440;
      }

    '';


  };
}
