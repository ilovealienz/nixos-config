# Sway: layout, colours, input, window rules and core keys. Each tool has
# its own file with its keys.
{ config, pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;
  mod = "Mod4";
  wmenuRun = "wmenu-run -f 'Inter 13' -N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.onAccent}";
  follow = config.my.placementFollow;
in
{
  wayland.windowManager.sway = {
    enable = true;

    config = {
      modifier = mod;
      terminal = "kitty";
      menu = wmenuRun;

      # mod + drag to move, mod + right-drag to resize
      floating.modifier = mod;

      gaps = {
        inner = 2;
        outer = 0;
      };

      window = {
        border = 2;
        titlebar = false;
      };
      floating = {
        border = 2;
        titlebar = false;
      };

      focus.followMouse = true;

      input = {
        "type:keyboard" = { xkb_layout = "gb"; };
        "type:pointer" = {
          accel_profile = "flat";
          pointer_accel = "0";
        };
      };

      # ── colours ──
      colors = {
        focused = {
          border = "#${c.accent}"; background = "#${c.accent}"; text = "#${c.onAccent}";
          indicator = "#${c.accent}"; childBorder = "#${c.accent}";
        };
        focusedInactive = {
          border = "#${c.surface}"; background = "#${c.surface}"; text = "#${c.fg}";
          indicator = "#${c.surface}"; childBorder = "#${c.surface}";
        };
        unfocused = {
          border = "#${c.surface}"; background = "#${c.bg}"; text = "#${c.muted}";
          indicator = "#${c.surface}"; childBorder = "#${c.surface}";
        };
        urgent = {
          border = "#${c.red}"; background = "#${c.red}"; text = "#${c.onAccent}";
          indicator = "#${c.red}"; childBorder = "#${c.red}";
        };
      };

      bars = [{ command = "${pkgs.waybar}/bin/waybar"; }];

      # wallpaper: wallpaper.nix
      startup = [
        { command = "${pkgs.polkit_gnome}/libexec/polkit-gnome-authentication-agent-1"; }
      ];

      window.commands = [
        { command = "floating enable, resize set 900 600, move position center";
          criteria = { app_id = "^(kitty|foot)-float$"; }; }
        { command = "floating enable"; criteria = { app_id = "pavucontrol"; }; }
        { command = "floating enable"; criteria = { app_id = "blueman-manager"; }; }
      ];

      keybindings = {

        # launching
        "${mod}+Shift+r" = "exec ${wmenuRun}";
        "${mod}+r" = "exec fuzzel";
        # follow to its workspace when placement is on
        "${mod}+Return" = "exec kitty; exec ${lib.getExe follow} 4";
        "${mod}+Shift+Return" = "exec foot; exec ${lib.getExe follow} 4";
        "${mod}+e" = "exec thunar; exec ${lib.getExe follow} 5";

        # window management
        "${mod}+q" = "kill";
        "${mod}+Shift+c" = "reload";
        "${mod}+Shift+e" = "exec session-menu";   # desktop/session.nix
        "${mod}+v" = "floating toggle";
        "${mod}+s" = "split toggle";

        # tabbed / stacked containers (more in sway-helpers.nix)
        "${mod}+Shift+g" = "layout toggle stacking tabbed";
        "${mod}+t" = "layout toggle split";

        # focus (vim keys)
        "${mod}+h" = "focus left";
        "${mod}+l" = "focus right";
        "${mod}+k" = "focus up";
        "${mod}+j" = "focus down";

        # move (up and down: sway-helpers.nix, for groups)
        "${mod}+Shift+h" = "move left";
        "${mod}+Shift+l" = "move right";

        # resize: moves the edge the key points to (the other edge if nothing's on that side)
        "${mod}+Alt+h" = "exec ${pkgs.sway}/bin/swaymsg -q resize shrink right 40px || ${pkgs.sway}/bin/swaymsg -q resize grow left 40px";
        "${mod}+Alt+l" = "exec ${pkgs.sway}/bin/swaymsg -q resize grow right 40px || ${pkgs.sway}/bin/swaymsg -q resize shrink left 40px";
        "${mod}+Alt+k" = "exec ${pkgs.sway}/bin/swaymsg -q resize shrink down 40px || ${pkgs.sway}/bin/swaymsg -q resize grow up 40px";
        "${mod}+Alt+j" = "exec ${pkgs.sway}/bin/swaymsg -q resize grow down 40px || ${pkgs.sway}/bin/swaymsg -q resize shrink up 40px";

        # workspaces
        "${mod}+1" = "workspace number 1";
        "${mod}+2" = "workspace number 2";
        "${mod}+3" = "workspace number 3";
        "${mod}+4" = "workspace number 4";
        "${mod}+5" = "workspace number 5";
        "${mod}+6" = "workspace number 6";
        "${mod}+7" = "workspace number 7";
        "${mod}+8" = "workspace number 8";
        "${mod}+9" = "workspace number 9";

        "${mod}+Shift+1" = "move container to workspace number 1";
        "${mod}+Shift+2" = "move container to workspace number 2";
        "${mod}+Shift+3" = "move container to workspace number 3";
        "${mod}+Shift+4" = "move container to workspace number 4";
        "${mod}+Shift+5" = "move container to workspace number 5";
        "${mod}+Shift+6" = "move container to workspace number 6";
        "${mod}+Shift+7" = "move container to workspace number 7";
        "${mod}+Shift+8" = "move container to workspace number 8";
        "${mod}+Shift+9" = "move container to workspace number 9";

        # move workspace between monitors
        "${mod}+Shift+comma"  = "move workspace to output left";
        "${mod}+Shift+period" = "move workspace to output right";
      };
    };

    extraConfig = ''
      # mark XWayland windows in the title
      for_window [shell="xwayland"] title_format "%title [XWayland]"
    '';
  };
}
