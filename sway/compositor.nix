{ pkgs, lib, osConfig, ... }:
let c = osConfig.theme.colors; in
let
  mod = "Mod4";

  # ── volume / brightness OSD via mako (replaces swayosd, ~119MB saved) ──
  osd = pkgs.writeShellScriptBin "osd" ''
    notify() {
      ${pkgs.libnotify}/bin/notify-send -c osd \
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
          ${pkgs.libnotify}/bin/notify-send -c osd -h string:x-canonical-private-synchronous:osd -t 1500 "volume" "muted"
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

  # ── screenshot: freeze → select → annotate → act ──────────────────
  # helper that runs INSIDE the freeze; separate script so we don't
  # have to self-reference via $0
  screenshot-capture = pkgs.writeShellScriptBin "screenshot-capture" ''
    out=$1
    geom=$(${pkgs.slurp}/bin/slurp -d 2>/dev/null)
    if [ -n "$geom" ]; then
      ${pkgs.grim}/bin/grim -g "$geom" "$out" 2>/dev/null || rm -f "$out"
    else
      rm -f "$out"                      # cancelled
    fi
    ${pkgs.procps}/bin/pkill -x wayfreeze 2>/dev/null
    exit 0
  '';

  screenshot-menu = pkgs.writeShellScriptBin "screenshot-menu" ''
    pics=$(${pkgs.xdg-user-dirs}/bin/xdg-user-dir PICTURES 2>/dev/null || echo "$HOME/Pictures")
    dir="$pics/Screenshots/$(date +%Y-%m)"
    mkdir -p "$dir"
    file="$dir/$(date +%Y-%m-%d_%H-%M-%S).png"

    # freeze the screen so nothing moves while you select
    ${pkgs.wayfreeze}/bin/wayfreeze --hide-cursor \
      --after-freeze-cmd "${screenshot-capture}/bin/screenshot-capture '$file'" \
      >/dev/null 2>&1
    ${pkgs.procps}/bin/pkill -x wayfreeze 2>/dev/null

    [ -s "$file" ] || exit 0            # cancelled, nothing captured

    MENU="${pkgs.wmenu}/bin/wmenu -f 'Inter 14' -N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.bg} -M ${c.accent} -m ${c.bg} -l 6 -p 'shot:'"

    actions='upload (zipline)
upload advanced
copy to clipboard
save only
open
delete'

    menu="annotate
$actions"

    choice=$(printf '%s' "$menu" | eval $MENU)

    if [ "$choice" = "annotate" ]; then
      before=$(stat -c %Y "$file" 2>/dev/null)
      ${pkgs.satty}/bin/satty --filename "$file" --fullscreen \
        --output-filename "$file" --early-exit \
        --copy-command ${pkgs.wl-clipboard}/bin/wl-copy
      after=$(stat -c %Y "$file" 2>/dev/null)
      # ctrl+s writes the file; ctrl+c only copies and exits without
      # touching it — so recover the annotated image from the clipboard
      if [ "$before" = "$after" ]; then
        tmp=$(mktemp -t satty-XXXXXX.png)
        if ${pkgs.wl-clipboard}/bin/wl-paste --type image/png > "$tmp" 2>/dev/null \
           && [ -s "$tmp" ]; then
          mv "$tmp" "$file"
        else
          rm -f "$tmp"
        fi
      fi
      [ -s "$file" ] || exit 0
      choice=$(printf '%s' "$menu" | eval $MENU)
    fi

    case "$choice" in
      "upload (zipline)")  "$HOME/.bin/zipline-upload" "$file" ;;
      "upload advanced")   "$HOME/.bin/zipline-upload" --advanced "$file" ;;
      "copy to clipboard") ${pkgs.wl-clipboard}/bin/wl-copy < "$file" \
                             && ${pkgs.libnotify}/bin/notify-send -c osd "screenshot" "copied" ;;
      "save only")         ${pkgs.libnotify}/bin/notify-send -c osd "screenshot" "saved to $file" ;;
      open)                ${pkgs.xdg-utils}/bin/xdg-open "$file" >/dev/null 2>&1 & ;;
      delete)              rm -f "$file" ;;
      *)                   : ;;
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
          | ${pkgs.wmenu}/bin/wmenu -f 'Inter 13' -N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.bg} -M ${c.accent} -m ${c.bg} -l 10 -p "missed:" )
        [ "$choice" = "clear all" ] && pkill -f 'bin/mako$'
        ;;
    esac
  '';

  # ── cycle tabs in the current tabbed/stacked group, wrapping around ──
  cycleGroup = pkgs.writeShellApplication {
    name = "sway-cycle-group";
    runtimeInputs = [ pkgs.jq pkgs.sway ];
    excludeShellChecks = [ "SC2016" ];
    text = ''
      # ── group-aware move / group toggle ──
      case "''${1:-}" in
        up|down|group)
          IFS=$'\t' read -r wid plo gplo tgt sibs < <(swaymsg -t get_tree | jq -r '
            . as $r
            | (first(paths(type == "object" and .focused == true and .type == "con")) // empty) as $p
            | select($p[-2] == "nodes")
            | ($r | getpath($p)) as $w
            | ($r | getpath($p[:-2])) as $par
            | (if ($p | length) >= 4 then ($r | getpath($p[:-4])).layout else "none" end) as $gplo
            | ([$par.nodes[] | select(.layout == "tabbed" or .layout == "stacked")] | first) as $grp
            | (if $grp then ([$grp.nodes[] | select((.nodes | length) == 0)] | first | .id // "-") else "-" end) as $tgt
            | (if $grp then [$par.nodes[] | select(.id != $grp.id and (.nodes | length) == 0) | .id | tostring] else [] end) as $sibs
            | [$w.id, $par.layout, $gplo, $tgt, (if ($sibs | length) > 0 then $sibs | join(",") else "-" end)] | @tsv') || exit 0

          ingroup() { [[ $plo == tabbed || $plo == stacked ]]; }

          case "$1" in
            up|down)
              if ingroup && [[ $gplo != splitv ]]; then
                swaymsg -q "focus parent; splitv; focus child; move $1"
              else
                swaymsg -q "move $1"
              fi ;;
            group)
              if ingroup; then
                swaymsg -q 'layout toggle tabbed split'
              elif [[ $tgt != - && $sibs != - ]]; then
                cmds="[con_id=$tgt] mark --add __grp"
                IFS=, read -ra ids <<< "$sibs"
                for s in "''${ids[@]}"; do cmds+="; [con_id=$s] move container to mark __grp"; done
                swaymsg -q "$cmds; unmark __grp; [con_id=$wid] focus"
              else
                swaymsg -q 'layout toggle tabbed split'
              fi ;;
          esac
          exit 0 ;;
      esac

      # ── Super+Tab: cycle tabs in the current group, wrapping ──
      dir=''${1:-1}
      id=$(swaymsg -t get_tree | jq -r --argjson d "$dir" '
        [.. | objects
          | select(.layout == "tabbed" or .layout == "stacked")
          | select([.nodes[] | .. | objects | select(.focused == true)] | length > 0)
        ] | last // empty
        | .nodes as $n
        | ($n | length) as $len
        | ($n | map([.. | objects | .focused] | any) | index(true)) as $i
        | $n[((($i + $d) % $len) + $len) % $len].id')
      if [ -n "$id" ]; then
        swaymsg -q "[con_id=$id] focus"
      fi
    '';
  };

in
{
  home.packages = [ osd screenshot-menu screenshot-capture dnd ];

  wayland.windowManager.sway = {
    enable = true;

    config = {
      modifier = mod;
      terminal = "kitty";
      menu = "wmenu-run -f 'Inter 13' -N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.bg}";

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
          border = "#${c.accent}"; background = "#${c.accent}"; text = "#${c.bg}";
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
          border = "#${c.red}"; background = "#${c.red}"; text = "#${c.bg}";
          indicator = "#${c.red}"; childBorder = "#${c.red}";
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

        # launching
        "${mod}+Shift+r" = "exec wmenu-run -f 'Inter 13' -N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.bg}";
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
        "${mod}+g" = "exec ${lib.getExe cycleGroup} group";
        "${mod}+t" = "layout toggle split";
        "${mod}+Tab" = "exec ${lib.getExe cycleGroup} 1";
        "${mod}+Shift+Tab" = "exec ${lib.getExe cycleGroup} -1";

        # focus (vim keys)
        "${mod}+h" = "focus left";
        "${mod}+l" = "focus right";
        "${mod}+k" = "focus up";
        "${mod}+j" = "focus down";

        # move
        "${mod}+Shift+h" = "move left";
        "${mod}+Shift+l" = "move right";
        "${mod}+Shift+k" = "exec ${lib.getExe cycleGroup} up";
        "${mod}+Shift+j" = "exec ${lib.getExe cycleGroup} down";

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

    services.wlsunset = {
    enable = true;
    latitude = "53.8";
    longitude = "-3.0";
  };

    systemd.user.services.wlsunset = {
    Unit = {
      After = lib.mkForce [ "sway-session.target" ];
      PartOf = lib.mkForce [ "sway-session.target" ];
    };
    Install.WantedBy = lib.mkForce [ "sway-session.target" ];
  };

  programs.swaylock = {
    enable = true;
    settings = {
      color = "${c.bg}";
      indicator-radius = 100;
      indicator-thickness = 10;
      ring-color = "${c.accent}";
      inside-color = "${c.surface}";
      text-color = "${c.fg}";
      key-hl-color = "${c.green}";
      line-color = "${c.bg}";
      separator-color = "${c.bg}";
      show-failed-attempts = true;
    };
  };
}
