# Window and group helpers.
#
# Small additions to sway, in one Rust program (standard library only,
# ~2.5MB of RAM in total). Each runs in its own thread and is restarted on
# its own if it crashes.
#
#   group-drag  Super+Shift+drag moves a whole tab group: drag any window
#               of the group and the rest follows it once it's dropped, in
#               the same order. Super+drag still moves a single window.
#   app-group   related apps join one tabbed group when they open; after
#               that they're left alone. Super+Alt+g pauses/resumes it.
#   group-end   a tab group down to one window is removed, leaving a plain
#               window, so new windows open beside it rather than as tabs:
#               at once if the other was closed (Super+q), 10 seconds
#               later if it was moved out or floated, so it can be put back.
#               A split left with one window after a close is removed too.
#   open-beside new windows never open underneath another: one that lands
#               in a vertical split (after Super+Shift+j/k, say) is moved
#               to the right of it. Super+s or Super+t first lets the next
#               one open below.
#   attention   XWayland apps (Wine/IMVU, Steam chats...) ask for attention
#               in a way sway ignores; the count is shown in the title,
#               e.g. "(3) the rock - IMVU", reset when you focus the window.
#               Super+U toggles also marking them urgent (red border + red
#               workspace). XWayland can still exit when no X11 app is open.
#
# Source: ../pkgs/sway-helpers/src/
#
# And two scripts:
#
#   fullscreen-toggle  Super+f; Proton games revert fullscreen while tiled,
#                      so they're floated first
#   sway-cycle-group   Super+Tab cycles the tabs of a group; Super+g groups
#                      a window with the ones beside it, or ungroups it;
#                      Super+Shift+j/k move up / down, out of a group if
#                      it's in one
{ pkgs, lib, ... }:
let
  # group name -> apps (Wayland app_id or XWayland class, case-insensitive).
  # To check a window: swaymsg -t get_tree | grep -E '"(app_id|class)"'
  groups = {
    chat = [ "vesktop" "signal" "ferdium" "org.telegram.desktop" "TelegramDesktop" ];
    minecraft = [ "org.prismlauncher.PrismLauncher" "com.mojang.minecraft" ];
  };

  # added after the title in every title attention sets; keep it the same
  # as the `title_format "%title [XWayland]"` rule in sway.nix
  titleSuffix = " [XWayland]";

  swayHelpers = pkgs.rustPlatform.buildRustPackage {
    pname = "sway-helpers";
    version = "1.2.0";
    src = ../pkgs/sway-helpers;
    cargoLock.lockFile = ../pkgs/sway-helpers/Cargo.lock;   # no dependencies
  };

  # flag files the program checks; each toggle flips one
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
  # the file pauses grouping, so creating it turns grouping off
  groupToggle = toggle "app-group-toggle" "app-group/off" "off" "on" "app grouping";
  urgentToggle = toggle "attention-urgent-toggle" "xwayland-attention/urgent" "on" "off" "xwayland urgent";

  # ── Super+f: Proton games revert fullscreen while tiled, so float
  # them first; pressing again returns them to tiling ──
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
        # one step at a time: Wine ignores size changes that arrive while
        # it is still handling the fullscreen state change
        swaymsg -q "[con_id=$id] fullscreen disable"
        sleep 0.2
        swaymsg -q "[con_id=$id] floating disable"
        sleep 0.2
        # resend the tile size in case Wine missed it: a border change
        # makes sway configure the window again
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
                # ungroup: always side by side (a toggle would bring back
                # whatever split the group was made from)
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

  # group-drag: Super then Shift, or Shift then Super (either side)
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
