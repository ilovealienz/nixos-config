{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  # screens narrower than this (logical px, after scaling) get the compact bar
  compactWidth = 1500;
  compact = {
    "sway/window".max-length = 25;
    cpu.format = "󰍛 {usage}%";
    memory.format = "󰘚 {percentage}%";
    pulseaudio = { format = "󰕾 {volume}%"; format-muted = "󰝟"; };
    network = {
      format-wifi = "󰖩";
      format-ethernet = "󰈀";
      format-disconnected = "󰖪";
      tooltip-format-wifi = "{essid}";
      tooltip-format-ethernet = "{ifname}";
    };
    # the date is in the calendar tooltip
    clock.format = "{:%H:%M}";
    "custom/tray-toggle".exec = "${lib.getExe trayToggle} status --compact";
  };

  # sway's bar command. Size (msqu, or waybar-start size MODE): auto gives
  # screens under compactWidth the compact bar, compact and full apply to all.
  # Kept in ~/.local/state/waybar/size; worked out again on each sway reload.
  waybarStart = pkgs.writeShellApplication {
    name = "waybar-start";
    runtimeInputs = [ pkgs.sway pkgs.jq pkgs.waybar ];
    text = ''
      cfg="''${XDG_CONFIG_HOME:-$HOME/.config}/waybar/config"
      run="''${XDG_RUNTIME_DIR:-/tmp}/waybar-config.json"
      state="''${XDG_STATE_HOME:-$HOME/.local/state}/waybar"

      mode() {
        case "$(cat "$state/size" 2>/dev/null || true)" in
          compact) echo compact ;;
          full) echo full ;;
          *) echo auto ;;
        esac
      }

      # the config waybar runs, for this size and these screens
      write() {
        local small
        case "$(mode)" in
          full) small='[]' ;;
          compact) small='"all"' ;;
          auto) small=$(swaymsg -r -t get_outputs | jq -c '[.[] | select(.active and .rect.width < ${toString compactWidth}) | .name]') || return 1 ;;
        esac
        # screens plugged in later get the full bar
        jq --argjson small "$small" --argjson compact '${builtins.toJSON compact}' '
          (if type == "array" then .[0] else . end) as $bar
          | if $small == "all" then [$bar * $compact]
            elif $small == [] then [$bar]
            else [ $bar + { output: (($small | map("!" + .)) + ["*"]) },
                   ($bar * $compact) + { output: $small } ]
            end' "$cfg" > "$run.tmp" || return 1
        mv "$run.tmp" "$run"
      }

      case "''${1:-}" in
        size)
          case "''${2:-}" in
            "") mode ;;
            auto | compact | full)
              mkdir -p "$state"
              echo "$2" > "$state/size"
              # sway restarts its bar on reload (waybar's own reload signal
              # crashes with several bars and the tray)
              swaymsg -q reload
              ;;
            *) echo "usage: waybar-start size [auto|compact|full]" >&2; exit 1 ;;
          esac
          ;;
        *)
          # drawers start closed
          rm -rf "''${XDG_RUNTIME_DIR:-/tmp}/waybar-tray"
          # sway adds -b <bar id>; the plain config if this one can't be made
          if write; then exec waybar -c "$run" "$@"; fi
          exec waybar "$@"
          ;;
      esac
    '';
  };

  # the tray drawer's toggle (../pkgs/tray-toggle): [3] (tray apps, names in
  # the tooltip) when closed, [<] when open, hidden with no tray apps.
  # Runs only when clicked or when tray apps come and go.
  trayToggle = pkgs.rustPlatform.buildRustPackage {
    pname = "tray-toggle";
    version = "1.0.0";
    src = ../pkgs/tray-toggle;
    cargoLock.lockFile = ../pkgs/tray-toggle/Cargo.lock;   # no dependencies
    buildInputs = [ pkgs.systemdLibs ];   # sd-bus
    meta.mainProgram = "tray-toggle";
  };
in
{
  wayland.windowManager.sway.config.bars = [{ command = lib.getExe waybarStart; }];
  # msqu sets the size with it
  home.packages = [ waybarStart ];

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
      # msqu.nix and birthdays.nix add theirs first
      modules-right = [ "custom/rec" "group/tray" "cpu" "memory" "pulseaudio" "network" "custom/weather" "clock" "battery" "custom/dnd" "custom/awake" ];

      "sway/workspaces" = {
        disable-scroll = true;
        all-outputs = true;
        format = "{icon}";
        # empty: every output
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

      # tray icons behind a toggle, click it to show them
      "group/tray" = {
        orientation = "inherit";
        drawer = {
          click-to-reveal = true;
          transition-duration = 250;
          transition-left-to-right = false;
          children-class = "tray-hidden";
        };
        modules = [ "custom/tray-toggle" "tray" ];
      };
      # the drawer opens on press, the toggle changes on release
      "custom/tray-toggle" = {
        return-type = "json";
        exec = "${lib.getExe trayToggle} status";
        # refreshed by tray-toggle (pkgs/tray-toggle/src/main.rs: SIGNAL)
        signal = 12;
        on-click-release = "${lib.getExe trayToggle} flip";
      };
      tray = {
        spacing = 10;
        icon-size = 16;
        # refreshes the toggle if the tray apps changed
        on-update = "${lib.getExe trayToggle} update";
        # clicks between icons would close the drawer behind the toggle's back
        on-click = "";
      };
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
        on-click = "power-menu";   # desktop/power.nix
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

      # awake.nix
      "custom/awake" = {
        return-type = "json";
        exec = "awake status";
        interval = 10;
        signal = 10;
        on-click = "awake left";
        on-click-right = "awake right";
        # the script refreshes the bar itself (waybar's raced it)
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

      /* the tray's group box is #tray too; its padding would double up */
      #tray:not(.module) {
        padding: 0;
      }
      #custom-tray-toggle {
        padding: 0 3px;
        color: #${c.fg};
      }
      #custom-tray-toggle:hover {
        color: #${c.accent};
      }

      /* awake sliver: amber = staying awake, green = game mode */
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

    '';
  };
}
