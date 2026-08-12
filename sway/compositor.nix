{ pkgs, ... }:
let
  mod = "Mod4";

  # ── volume / brightness OSD via mako (replaces swayosd, ~119MB saved) ──
  osd = pkgs.writeShellScriptBin "osd" ''
    notify() {
      ${pkgs.libnotify}/bin/notify-send \
        -h int:value:"$2" \
        -h string:x-canonical-private-synchronous:osd \
        -t 1500 "$1" "$2%"
    }

    case "$1" in
      vol-up)
        ${pkgs.wireplumber}/bin/wpctl set-volume -l 1.5 @DEFAULT_AUDIO_SINK@ 5%+
        notify "volume" "$(${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gawk}/bin/awk '{print int($2*100)}')" ;;
      vol-down)
        ${pkgs.wireplumber}/bin/wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-
        notify "volume" "$(${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gawk}/bin/awk '{print int($2*100)}')" ;;
      vol-mute)
        ${pkgs.wireplumber}/bin/wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
        if ${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gnugrep}/bin/grep -q MUTED; then
          ${pkgs.libnotify}/bin/notify-send -h string:x-canonical-private-synchronous:osd -t 1500 "volume" "muted"
        else
          notify "volume" "$(${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SINK@ | ${pkgs.gawk}/bin/awk '{print int($2*100)}')"
        fi ;;
      bright-up)
        ${pkgs.brightnessctl}/bin/brightnessctl set 5%+ >/dev/null
        notify "brightness" "$(${pkgs.brightnessctl}/bin/brightnessctl -m | ${pkgs.coreutils}/bin/cut -d, -f4 | ${pkgs.gnused}/bin/sed 's/%//')" ;;
      bright-down)
        ${pkgs.brightnessctl}/bin/brightnessctl set 5%- >/dev/null
        notify "brightness" "$(${pkgs.brightnessctl}/bin/brightnessctl -m | ${pkgs.coreutils}/bin/cut -d, -f4 | ${pkgs.gnused}/bin/sed 's/%//')" ;;
    esac
  '';

  screenshot-menu = pkgs.writeShellScriptBin "screenshot-menu" ''
    pics="$(${pkgs.xdg-user-dirs}/bin/xdg-user-dir PICTURES)"
    dir="$pics/Screenshots/$(date +%Y-%m)"
    mkdir -p "$dir"
    file="$dir/$(date +%Y-%m-%d_%H-%M-%S).png"

    ${pkgs.grim}/bin/grim -g "$(${pkgs.slurp}/bin/slurp)" "$file" || exit 0

    choice=$(printf "open\ncopy\nupload (zipline)\nupload advanced\ndelete" \
      | ${pkgs.wmenu}/bin/wmenu -f 'Inter 14' -N 24221c -n d4b07b -S e5a440 -s 24221c -M e5a440 -m 24221c -p "shot:")

    case "$choice" in
      open)               xdg-open "$file" ;;
      copy)               ${pkgs.wl-clipboard}/bin/wl-copy < "$file" ;;
      "upload (zipline)") $HOME/.bin/zipline-upload "$file" ;;
      "upload advanced")  $HOME/.bin/zipline-upload --advanced "$file" ;;
      delete)             rm "$file" ;;
    esac
  '';

  dnd = pkgs.writeShellScriptBin "dnd" ''
    case "$1" in
      toggle)
        ${pkgs.mako}/bin/makoctl mode -t dnd >/dev/null
        pkill -RTMIN+9 waybar
        ;;
      status)
        if ${pkgs.mako}/bin/makoctl mode | ${pkgs.gnugrep}/bin/grep -q '^dnd$'; then
	  printf '{"text":"\Uf009a","class":"dnd","tooltip":"do not disturb"}\n'
        else
          printf '{"text":"\Uf009a","class":"active","tooltip":"notifications on"}\n'
        fi
        ;;
       history)
        choice=$( { printf 'clear all\n'; \
          ${pkgs.mako}/bin/makoctl history -j \
          | ${pkgs.jq}/bin/jq -r '.[] | "\(.id) \(.app_name): \(.summary) — \(.body)"'; } \
          | ${pkgs.wmenu}/bin/wmenu -f 'Inter 13' -N 24221c -n d4b07b -S e5a440 -s 24221c -M e5a440 -m 24221c -l 10 -p "missed:" )
        [ "$choice" = "clear all" ] && pkill -f 'bin/mako$'
        ;;
    esac
  '';

in
{
  home.packages = [ osd screenshot-menu dnd ];

  wayland.windowManager.sway = {
    enable = true;

    config = {
      modifier = mod;
      terminal = "kitty";
      menu = "wmenu-run -f 'Inter 13' -N 24221c -n d4b07b -S e5a440 -s 24221c";

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

      # wallpaper — sway does this natively, no swaybg process
      output."*".bg = "${../walls/1.png} fill";

      # ── desert night colours ──
      colors = {
        focused = {
          border = "#e5a440"; background = "#e5a440"; text = "#24221c";
          indicator = "#e5a440"; childBorder = "#e5a440";
        };
        focusedInactive = {
          border = "#473f31"; background = "#473f31"; text = "#d4b07b";
          indicator = "#473f31"; childBorder = "#473f31";
        };
        unfocused = {
          border = "#473f31"; background = "#24221c"; text = "#87765d";
          indicator = "#473f31"; childBorder = "#473f31";
        };
        urgent = {
          border = "#e56b55"; background = "#e56b55"; text = "#24221c";
          indicator = "#e56b55"; childBorder = "#e56b55";
        };
      };

      bars = [{ command = "${pkgs.waybar}/bin/waybar"; }];

      startup = [
        { command = "${pkgs.polkit_gnome}/libexec/polkit-gnome-authentication-agent-1"; }
      ];

      # ── app → workspace ──
      # NOTE: verify these with `swaymsg -t get_tree | grep -E 'app_id|class'`
      # sway uses app_id for wayland apps, class for xwayland.
      assigns = {
        "1" = [ { app_id = "firefox"; } { app_id = "brave-browser"; } { app_id = "floorp"; } ];
        "2" = [ { app_id = "spotify"; } { app_id = "signal"; } { app_id = "vesktop"; } ];
        "3" = [ { app_id = "mpv"; } ];
        "4" = [ { app_id = "kitty"; } ];
        "5" = [ { app_id = "thunar"; } ];
        "6" = [ { app_id = "org.qbittorrent.qBittorrent"; } ];
        "7" = [ { app_id = "virt-manager"; } ];
      };

      window.commands = [
        { command = "floating enable, resize set 900 600, move position center";
          criteria = { app_id = "kitty-float"; }; }
        { command = "floating enable"; criteria = { app_id = "pavucontrol"; }; }
        { command = "floating enable"; criteria = { app_id = "blueman-manager"; }; }
      ];

      keybindings = {

	"${mod}+Tab" = "focus next";
        "${mod}+Shift+Tab" = "focus prev";
	
        # launching
        "${mod}+Shift+r" = "exec wmenu-run -f 'Inter 13' -N 24221c -n d4b07b -S e5a440 -s 24221c";
        "${mod}+r" = "exec fuzzel";
        "${mod}+Shift+x" = "exec swaylock";
	"${mod}+Return" = "exec kitty; workspace number 4";
        "${mod}+e" = "exec thunar; workspace number 5";

        # window management
        "${mod}+q" = "kill";
        "${mod}+Shift+e" = "exec swaynag -t warning -m 'exit sway?' -B 'yes' 'swaymsg exit'";
        "${mod}+f" = "fullscreen toggle";
        "${mod}+v" = "floating toggle";
        "${mod}+s" = "split toggle";

        # tabbed / stacked containers (replaces hyprland groups)
        "${mod}+g" = "layout toggle tabbed split";
        "${mod}+t" = "layout toggle split";

        # focus (vim keys)
        "${mod}+h" = "focus left";
        "${mod}+l" = "focus right";
        "${mod}+k" = "focus up";
        "${mod}+j" = "focus down";

        # move
        "${mod}+Shift+h" = "move left";
        "${mod}+Shift+l" = "move right";
        "${mod}+Shift+k" = "move up";
        "${mod}+Shift+j" = "move down";

        # resize
        "${mod}+Alt+h" = "resize shrink width 40px";
        "${mod}+Alt+l" = "resize grow width 40px";
        "${mod}+Alt+k" = "resize shrink height 40px";
        "${mod}+Alt+j" = "resize grow height 40px";

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

        # screenshots
        "${mod}+Shift+s" = "exec ${pkgs.grim}/bin/grim -g \"$(${pkgs.slurp}/bin/slurp)\" - | ${pkgs.wl-clipboard}/bin/wl-copy";
        "Print" = "exec screenshot-menu";

        # media / volume / brightness
        "XF86AudioRaiseVolume" = "exec osd vol-up";
        "XF86AudioLowerVolume" = "exec osd vol-down";
        "XF86AudioMute" = "exec osd vol-mute";
        "XF86MonBrightnessUp" = "exec osd bright-up";
        "XF86MonBrightnessDown" = "exec osd bright-down";
        "XF86AudioPlay" = "exec ${pkgs.playerctl}/bin/playerctl play-pause";
        "XF86AudioNext" = "exec ${pkgs.playerctl}/bin/playerctl next";
        "XF86AudioPrev" = "exec ${pkgs.playerctl}/bin/playerctl previous";
      };
    };

    extraConfig = ''
      # don't let sway steal these from fullscreen apps
      for_window [shell="xwayland"] title_format "%title [XWayland]"
    '';
  };

  # ── idle: lock at 10min, screens off at 15 ──
  services.swayidle = {
    enable = true;
    timeouts = [
      { timeout = 600; command = "${pkgs.swaylock}/bin/swaylock -f"; }
      {
        timeout = 900;
        command = "${pkgs.sway}/bin/swaymsg 'output * power off'";
        resumeCommand = "${pkgs.sway}/bin/swaymsg 'output * power on'";
      }
    ];
  };

  programs.swaylock = {
    enable = true;
    settings = {
      color = "24221c";
      indicator-radius = 100;
      indicator-thickness = 10;
      ring-color = "e5a440";
      inside-color = "473f31";
      text-color = "d4b07b";
      key-hl-color = "99b05f";
      line-color = "24221c";
      separator-color = "24221c";
      show-failed-attempts = true;
    };
  };
}
