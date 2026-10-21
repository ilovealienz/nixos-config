# Window and group helpers, one Rust program (../pkgs/sway-helpers/src/):
#
#   group-drag   Super+Shift+drag moves a whole tab group
#   app-group    related apps open as one tab group (Super+Alt+g toggles)
#   group-end    a tab group of one becomes a plain window
#   open-beside  new windows open beside, not below (Super+s/t to override)
#   attention    XWayland attention requests shown as a count in the title
#                (Super+u also marks them urgent)
#
# Scripts: fullscreen-toggle (Super+f) and sway-cycle-group (Super+Tab,
# Super+g, Super+Shift+j/k).
{ pkgs, lib, ... }:
let
  # group -> app_ids or XWayland classes (any case)
  groups = {
    chat = [ "vesktop" "signal" "ferdium" "org.telegram.desktop" "TelegramDesktop" ];
    minecraft = [ "org.prismlauncher.PrismLauncher" "com.mojang.minecraft" ];
  };

  # must match the title_format rule in sway.nix
  titleSuffix = " [XWayland]";

  swayHelpers = pkgs.rustPlatform.buildRustPackage {
    pname = "sway-helpers";
    version = "1.2.0";
    src = ../pkgs/sway-helpers;
    cargoLock.lockFile = ../pkgs/sway-helpers/Cargo.lock;   # no dependencies
  };

  # each toggle flips a flag file
  toggle = name: flag: on: off: title: pkgs.writeShellApplication {
    inherit name;
    runtimeInputs = [ pkgs.coreutils pkgs.libnotify ];
    text = ''
      flag="''${XDG_STATE_HOME:-$HOME/.local/state}/${flag}"
      mkdir -p "$(dirname "$flag")"
      if [ -e "$flag" ]; then
        rm "$flag"; state="${off}"
      else
        touch "$flag"; state="${on}"
      fi
      notify-send -c osd -h string:x-canonical-private-synchronous:osd \
        -t 1500 "${title}" "$state"
    '';
  };
  groupToggle = toggle "app-group-toggle" "app-group/off" "off" "on" "app grouping";
  urgentToggle = toggle "attention-urgent-toggle" "xwayland-attention/urgent" "on" "off" "xwayland urgent";

  # ── Super+f: Proton games are floated first (they revert fullscreen tiled) ──
  fsToggle = pkgs.writeShellApplication {
    name = "fullscreen-toggle";
    runtimeInputs = [ pkgs.jq pkgs.sway ];
    text = ''
      read -r id fs class border width < <(swaymsg -t get_tree | jq -r '
        first(.. | objects | select(.focused == true))
        | "\(.id) \(.fullscreen_mode) \(.window_properties.class // "-") \(.border) \(.current_border_width)"')

      case "$class" in
        steam_app_*|steam_proton) ;;
        *) swaymsg -q fullscreen toggle; exit 0 ;;
      esac

      if [ "$fs" != 0 ]; then
        # one step at a time: Wine drops changes mid-transition
        swaymsg -q "[con_id=$id] fullscreen disable"
        sleep 0.2
        swaymsg -q "[con_id=$id] floating disable"
        sleep 0.2
        # a border change resends the size
        [ "$border" = none ] && other=pixel || other=none
        case "$border" in normal|pixel) restore="$border $width" ;; *) restore=$border ;; esac
        swaymsg -q "[con_id=$id] border $other"
        swaymsg -q "[con_id=$id] border $restore"
      else
        swaymsg -q "[con_id=$id] floating enable"
        sleep 0.2   # let the game settle at its own size first
        swaymsg -q "[con_id=$id] fullscreen enable"
      fi
    '';
  };

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
                # ungroup side by side
                swaymsg -q 'layout splith'
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

  args = lib.concatMap
    (name: [ "--group" "${name}=${lib.concatStringsSep "," groups.${name}}" ])
    (lib.attrNames groups)
    ++ [ "--title-suffix" titleSuffix ];
in
{
  systemd.user.services.sway-helpers = {
    Unit = {
      Description = "Tab group dragging, app grouping, ending groups of one, opening windows beside and XWayland attention for sway";
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = "${swayHelpers}/bin/sway-helpers ${lib.escapeShellArgs args}";
      Restart = "on-failure";
      RestartSec = 3;
    };
    Install.WantedBy = [ "sway-session.target" ];
  };

  wayland.windowManager.sway.config.keybindings = {
    "Mod4+Alt+g" = "exec ${lib.getExe groupToggle}";
    "Mod4+u" = "exec ${lib.getExe urgentToggle}";

    "Mod4+f" = "exec ${lib.getExe fsToggle}";
    "Mod4+g" = "exec ${lib.getExe cycleGroup} group";
    "Mod4+Tab" = "exec ${lib.getExe cycleGroup} 1";
    "Mod4+Shift+k" = "exec ${lib.getExe cycleGroup} up";
    "Mod4+Shift+j" = "exec ${lib.getExe cycleGroup} down";
  };

  # group-drag: Super and Shift in either order
  wayland.windowManager.sway.extraConfig = ''
    bindsym --no-repeat Mod4+Shift_L nop group-drag
    bindsym --no-repeat Mod4+Shift_R nop group-drag
    bindsym --no-repeat Shift+Super_L nop group-drag
    bindsym --no-repeat Shift+Super_R nop group-drag
    bindsym --release Mod4+Shift_L nop group-drag-off
    bindsym --release Mod4+Shift_R nop group-drag-off
    bindsym --release Shift+Super_L nop group-drag-off
    bindsym --release Shift+Super_R nop group-drag-off
  '';
}
