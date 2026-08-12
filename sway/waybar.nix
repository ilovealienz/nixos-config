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

      modules-left = [ "sway/workspaces" ];
      modules-center = [ "sway/window" ];
      modules-right = [ "tray" "cpu" "memory" "pulseaudio" "network" "custom/weather" "clock" "battery" "custom/dnd" "idle_inhibitor" ];

      "sway/workspaces" = {
        disable-scroll = true;
        all-outputs = true;
        format = "{icon}";
        # empty list = show on every output
        persistent-workspaces = {
          "1" = [];
        };
        format-icons = {
         "1" = "";  # firefox
         "2" = "󰭹";  # chat
         "3" = "";  # play
         "4" = "";  # terminal
         "5" = "";  # folder
         "6" = "";   # magnet
         "7" = "󰍺";  # monitor
         #urgent = "\Uf0026";
         #default = "\Uf02fc";
        };
      };

      "sway/window" = { max-length = 60; separate-outputs = true; };

      tray = { spacing = 10; icon-size = 16; };
      cpu = { format = "[CPU: {usage}%]"; interval = 5; };
      memory = { format = "[RAM: {percentage}%]"; interval = 5; };
      pulseaudio = {
        format = "[VOL: {volume}%]";
        format-muted = "muted";
        on-click = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle";
        on-click-right = "pavucontrol";
      };
      network = {
        format-wifi = "[{essid}]";
        format-ethernet = "[{ifname}]";
        format-disconnected = "offline";
        on-click = "kitty --class kitty-float -e nmtui";
        on-click-right = "blueman-manager";
      };
      battery = {
        states = { warning = 30; critical = 15; };
        format = "{capacity}%";
        format-charging = "{capacity}%+";
        format-plugged = "{capacity}%p";
        tooltip-format = "{timeTo}";
      };
      clock = {
        format = "{:%a %d %b  %H:%M}";
        tooltip-format = "<tt>{calendar}</tt>";
        calendar = {
          mode = "month";
          mode-mon-col = 3;
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

      "custom/dnd" = {
        return-type = "json";
        interval = 5;
        signal = 9;
        exec = "dnd status";
        on-click = "dnd history";
        on-click-right = "dnd toggle";
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
        min-width: 24px;
        padding: 0 7px;
        background: #24221c;
        color: #d4b07b;                      /* has windows */
        border-bottom: 2px solid transparent;
      }
      #workspaces button.focused {
        color: #e5a440;                      /* focused: amber */
        border-bottom: 2px solid #e5a440;
      }

      #workspaces button.urgent {
        background: #e56b55;
        color: #24221c;
      }

      #window { color: #87765d; }

      #tray, #cpu, #memory, #pulseaudio, #network, #battery, #clock {
        padding: 0 3px;
        color: #d4b07b;
      }

      /* thin toggle sliver — dim = idle on, amber = idle paused */
      #idle_inhibitor {
        min-width: 3px;
        margin: 0 0 0 4px;
        padding: 0;
        background: #473f31;
      }
      #idle_inhibitor.activated {
        background: #e5a440;
      }

      #custom-dnd {
        padding: 0 8px;
        color: #d4b07b;
      }
      #custom-dnd.dnd {
        color: #806b68;
      }

    '';
  };
}
