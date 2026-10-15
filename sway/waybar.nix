{ pkgs, osConfig, ... }:
let c = osConfig.theme.colors; in
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
      modules-right = [ "custom/rec" "tray" "cpu" "memory" "pulseaudio" "network" "custom/weather" "clock" "battery" "custom/dnd" "custom/awake" ];

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
        on-click = "power-menu";   # sway/power.nix
      };
      clock = {
        format = "{:%a %d %b  %H:%M}";
        tooltip-format = "<tt>{calendar}</tt>";
        calendar = {
          mode = "month";
          mode-mon-col = 3;
          weeks-pos = "right";
          format = {
            months    = "<span color='#${c.accent}'><b>{}</b></span>";
            days      = "<span color='#${c.fg}'>{}</span>";
            weeks     = "<span color='#${c.muted}'>W{}</span>";
            weekdays  = "<span color='#${c.orange}'><b>{}</b></span>";
            today     = "<span color='#${c.red}'><b><u>{}</u></b></span>";
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

      "custom/rec" = {
        return-type = "json";
        interval = 1;
        signal = 8;
        exec = "screenrec status";
        on-click = "screenrec";
      };      

      # left-click stay awake, right-click game mode (sway/awake.nix)
      "custom/awake" = {
        return-type = "json";
        exec = "awake status";
        interval = 10;
        signal = 10;
        on-click = "awake left";
        on-click-right = "awake right";
        # the click script refreshes the bar itself; waybar's own refresh
        # on click raced it and showed the old state
        exec-on-event = false;
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
        background: #${c.bg};
        color: #${c.fg};
      }

      tooltip {
        background-color: #${c.bg};
        border: 0px solid #${c.accent};
      }
      tooltip label {
        color: #${c.fg};
      }

      #workspaces button {
        font-family: "MonaspiceAr Nerd Font", monospace;
        font-size: 15px;
        min-width: 24px;
        padding: 0 7px;
        background: #${c.bg};
        color: #${c.fg};                      /* has windows */
        border-bottom: 2px solid transparent;
      }
      #workspaces button.focused {
        color: #${c.accent};                      /* focused: amber */
        border-bottom: 2px solid #${c.accent};
      }

      #workspaces button.urgent {
        background: #${c.red};
        color: #${c.onAccent};
      }

      #window { color: #${c.muted}; }

      #tray, #cpu, #memory, #pulseaudio, #network, #battery, #clock {
        padding: 0 3px;
        color: #${c.fg};
      }

      /* thin toggle sliver — dim = idle on, amber = staying awake,
         green = game mode */
      #custom-awake {
        min-width: 3px;
        margin: 0 0 0 4px;
        padding: 0;
        background: #${c.surface};
      }
      #custom-awake.awake {
        background: #${c.accent};
      }
      #custom-awake.game {
        background: #${c.green};
      }

      #custom-dnd {
        padding: 0 8px;
        color: #${c.fg};
      }
      #custom-dnd.dnd {
        color: #${c.red};
        background: alpha(#${c.red}, 0.15);
      }

      #custom-rec {
        padding: 0 8px;
        color: #${c.onAccent};
        background: #${c.red};
      }
      #custom-rec.busy {
        background: #${c.orange};
      }

    '';
  };
}
