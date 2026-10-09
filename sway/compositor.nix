{ config, pkgs, lib, osConfig, ... }:
let c = osConfig.theme.colors; in
let
  mod = "Mod4";

  # ── volume / brightness OSD via mako (replaces swayosd, ~119MB saved) ──
  # ── Super+;: emoji picker (bemoji in fuzzel), recent emoji first.
  # Enter puts the emoji into the focused window, Shift+Enter only copies
  # it. It's pasted rather than typed: wtype types emoji as the wrong
  # characters in Electron apps and XWayland windows, but a Ctrl+V
  # keypress works everywhere (sent with xdotool to XWayland windows).
  # Uses the window picker's fuzzel config, where Shift+Enter is custom-1 ──
  # Emoji list built from Unicode's data file, so bemoji never downloads
  # (a failed download leaves an empty list it never retries)
  emojiDb = pkgs.runCommand "bemoji-emoji-list" { } ''
    mkdir -p $out
    sed -ne 's/^.*; fully-qualified.*# \(\S*\) \S* \(.*$\)/\1 \2/gp' \
      ${pkgs.unicode-emoji}/share/unicode/emoji/emoji-test.txt > $out/emojis.txt
    [ -s $out/emojis.txt ]   # fail the build rather than ship an empty list
  '';

  emojiPicker = pkgs.writeShellApplication {
    name = "emoji-picker";
    runtimeInputs = with pkgs; [ bemoji fuzzel wl-clipboard wtype xdotool libnotify jq sway coreutils gnugrep gnused ];
    text = ''
      export BEMOJI_DB_LOCATION=${emojiDb}
      # grep . drops the blank line bemoji adds before the recent list
      export BEMOJI_PICKER_CMD="grep . | fuzzel --dmenu --config ${pickerFuzzel} --prompt 'emoji: ' --width 50 --placeholder 'enter: type · shift+enter: copy'"
      # bemoji copies on fuzzel's custom-1 (Shift+Enter); make that print
      # the emoji with a "copy:" prefix instead, so this script handles it
      export BEMOJI_CLIP_CMD="sed s/^/copy:/"
      export BEMOJI_TYPE_CMD=true   # bemoji's own typing (Alt+2): no-op

      out=$(bemoji -e -n) || exit 0   # -e: print it, -n: no newline
      [ -n "$out" ] || exit 0
      emoji=''${out#copy:}
      printf '%s' "$emoji" | wl-copy

      if [ "$out" != "$emoji" ]; then   # Shift+Enter
        # osd category: not kept in mako's history
        notify-send -c osd -h string:x-canonical-private-synchronous:osd \
          -t 1500 "copied" "$emoji"
        exit 0
      fi

      shell=- app=-
      read -r shell app < <(swaymsg -t get_tree | jq -r '
        first(.. | objects | select(.focused == true))
        | "\(.shell // "-") \(.app_id // .window_properties.class // "-")"') || true
      # let keyboard focus return from fuzzel; XWayland takes a moment
      # longer to hand X focus back to the window
      sleep 0.15
      if [ "$shell" = xwayland ]; then
        # X11 windows (Proton, Bottles/Wine, Brave): send the key through
        # the X server; wtype's keys don't reliably reach XWayland
        xdotool key --clearmodifiers ctrl+v
      else
        case "$app" in
          # terminals paste with Ctrl+Shift+V
          kitty|kitty-float|foot|footclient|Alacritty|org.wezfurlong.wezterm|com.mitchellh.ghostty)
            wtype -M ctrl -M shift -k v -m shift -m ctrl ;;
          *) wtype -M ctrl -k v -m ctrl ;;
        esac
      fi
    '';
  };

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

  # ── Super+a: audio menu. The current output and mic get their own
  # section at the top. Enter on another output or mic makes it the
  # default (playing audio moves with it); Enter on the current mic
  # toggles its mute; Shift+Enter opens hide / rename / reset name.
  # Reopens after each change. Settings in ~/.config/uwuaudio, both written
  # by the menu: hidden (one device name per line) and names
  # ("device.name = Nickname").
  # Uses the window picker's fuzzel config for Shift+Enter (custom-1) ──
  audioMenu = pkgs.writeShellApplication {
    name = "audio-menu";
    runtimeInputs = [ pkgs.pulseaudio pkgs.jq pkgs.fuzzel pkgs.coreutils pkgs.gnugrep pkgs.gawk ];
    text = ''
      cfg="''${XDG_CONFIG_HOME:-$HOME/.config}/uwuaudio"
      hidden="$cfg/hidden" names="$cfg/names"
      mkdir -p "$cfg"; touch "$hidden"

      # nickname from the names file, else the device's own description
      # names file: "device.name = Nickname", split at the first "="
      nickname() {
        [ -f "$names" ] || return 0
        awk -v k="$1" '{ i = index($0, "="); n = substr($0, 1, i - 1); v = substr($0, i + 1)
          gsub(/^ +| +$/, "", n); gsub(/^ +| +$/, "", v)
          if (i && n == k) { print v; exit } }' "$names"
      }
      drop_nickname() {
        awk -v k="$1" '{ i = index($0, "="); n = substr($0, 1, i - 1)
          gsub(/^ +| +$/, "", n); if (!i || n != k) print }' "$names" > "$names.tmp"
        mv "$names.tmp" "$names"
      }
      label() {
        local n
        n=$(nickname "$1")
        printf '%s' "''${n:-$2}"
      }
      # fuzzel's output is a row number only if a row was picked
      valid() { [[ $1 =~ ^[0-9]+$ ]] && (( $1 < $2 )); }
      menu() {
        fuzzel --dmenu --index --config ${pickerFuzzel} --width 60 \
          --font 'MonaspiceAr Nerd Font:size=11' "$@"
      }

      while :; do
        sink=$(pactl get-default-sink)
        source=$(pactl get-default-source)
        sinks=$(pactl -f json list sinks)
        sources=$(pactl -f json list sources \
          | jq 'map(select(.name | endswith(".monitor") | not))')
        desc() { jq -r --arg n "$2" '.[] | select(.name == $n) | .description // .name' <<< "$1"; }
        muted=$(jq -r --arg n "$source" '.[] | select(.name == $n) | .mute' <<< "$sources")

        lines=() acts=()
        lines+=("── current ─────────────────────────────") acts+=("-")
        d=$(desc "$sinks" "$sink")
        lines+=("   output  $(label "$sink" "''${d:-$sink}")") acts+=("cursink $sink")
        tag=""; [ "$muted" = true ] && tag="   [muted]"
        d=$(desc "$sources" "$source")
        lines+=("   mic     $(label "$source" "''${d:-$source}")$tag") acts+=("mute $source")

        lines+=("── outputs ─────────────────────────────") acts+=("-")
        while IFS=$'\t' read -r name d; do
          grep -Fxq -- "$name" "$hidden" && continue
          lines+=("   $(label "$name" "$d")") acts+=("sink $name")
        done < <(jq -r --arg n "$sink" '.[] | select(.name != $n) | [.name, .description // .name] | @tsv' <<< "$sinks")

        lines+=("── mics ────────────────────────────────") acts+=("-")
        while IFS=$'\t' read -r name d; do
          grep -Fxq -- "$name" "$hidden" && continue
          lines+=("   $(label "$name" "$d")") acts+=("source $name")
        done < <(jq -r --arg n "$source" '.[] | select(.name != $n) | [.name, .description // .name] | @tsv' <<< "$sources")

        count=$(grep -c . "$hidden" || true)
        if [ "$count" -gt 0 ]; then
          lines+=("   show hidden ($count)") acts+=("hidden -")
        fi

        rc=0
        i=$(printf '%s\n' "''${lines[@]}" | menu --prompt 'audio: ' \
              --placeholder 'enter: switch · shift+enter: hide/rename · current mic: mute') || rc=$?
        [ -n "$i" ] || exit 0
        valid "$i" "''${#acts[@]}" || continue   # nothing matched the search

        read -r kind name <<< "''${acts[$i]}"
        if [ "$rc" = 10 ]; then   # Shift+Enter: device options
          case "$kind" in sink|source|mute|cursink) ;; *) continue ;; esac
          opts=("rename")
          case "$kind" in sink|source) opts=("hide" "rename") ;; esac   # current devices can't be hidden
          [ -n "$(nickname "$name")" ] && opts+=("reset name")
          j=$(printf '%s\n' "''${opts[@]}" | menu --prompt 'device: ') || continue
          valid "$j" "''${#opts[@]}" || continue
          case "''${opts[$j]}" in
            hide)
              [ -s "$hidden" ] && [ -n "$(tail -c1 "$hidden")" ] && echo >> "$hidden"
              echo "$name" >> "$hidden" ;;
            rename)
              # no entries: whatever is typed is printed on Enter
              new=$(fuzzel --dmenu --width 60 --font 'MonaspiceAr Nerd Font:size=11' \
                      --prompt 'new name: ' --placeholder 'type a name, enter to save' \
                      < /dev/null) || continue
              [ -n "$new" ] || continue
              touch "$names"
              drop_nickname "$name"
              printf '%s = %s\n' "$name" "$new" >> "$names" ;;
            "reset name") drop_nickname "$name" ;;
          esac
          continue
        fi
        case "$kind" in
          sink)
            # move what was playing on the old default, so calls and videos
            # switch too; streams routed elsewhere on purpose stay put
            old=$(jq -r --arg n "$sink" '.[] | select(.name == $n) | .index' <<< "$sinks")
            pactl set-default-sink "$name"
            pactl list short sink-inputs | while read -r id on _; do
              if [ "$on" = "$old" ]; then pactl move-sink-input "$id" "$name" || true; fi
            done ;;
          source)
            # same for recording: only streams on the old default mic, so a
            # screen recorder capturing desktop audio isn't moved to a mic
            old=$(jq -r --arg n "$source" '.[] | select(.name == $n) | .index' <<< "$sources")
            pactl set-default-source "$name"
            pactl list short source-outputs | while read -r id on _; do
              if [ "$on" = "$old" ]; then pactl move-source-output "$id" "$name" || true; fi
            done ;;
          mute) pactl set-source-mute "$name" toggle ;;
          hidden)
            # Enter on a hidden device shows it again
            all=$(jq -s 'add' <<< "$sinks$sources")
            mapfile -t hnames < <(grep . "$hidden")
            hl=()
            for n in "''${hnames[@]}"; do
              d=$(desc "$all" "$n"); hl+=("   $(label "$n" "''${d:-$n}")")
            done
            j=$(printf '%s\n' "''${hl[@]}" | menu --prompt 'hidden: ' \
                  --placeholder 'enter: show it again') || continue
            valid "$j" "''${#hnames[@]}" || continue
            grep -Fxv -- "''${hnames[$j]}" "$hidden" > "$hidden.tmp" || true
            mv "$hidden.tmp" "$hidden" ;;
        esac
      done
    '';
  };

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

  # ── screen recording: select region → record → act ───────────────
  # Toggle: first run starts, second run (or clicking the bar module)
  # stops. Records desktop audio only, via the default sink's monitor.
  # wl-screenrec encodes on the GPU (VAAPI) and falls back to a software
  # encoder when that fails, e.g. no VAAPI driver.
  # Recording process name, used for pgrep/pkill:
  #   wl-screenrec (current) and wf-recorder (older versions of this script)
  screenrec = pkgs.writeShellScriptBin "screenrec" ''
    state="''${XDG_RUNTIME_DIR:-/tmp}/screenrec.state"
    running() { ${pkgs.procps}/bin/pgrep -x 'wl-screenrec|wf-recorder' >/dev/null; }
    sig() { ${pkgs.procps}/bin/pkill "$1" -x 'wl-screenrec|wf-recorder'; }
    bar() { ${pkgs.procps}/bin/pkill -RTMIN+8 waybar; }

    # ── waybar module ──
    if [ "''${1:-}" = status ]; then
      if running && [ -f "$state" ]; then
        s=$(( $(date +%s) - $(stat -c %Y "$state") ))
        printf '{"text":"● REC %02d:%02d","class":"recording","tooltip":"click to stop"}\n' \
          $((s / 60)) $((s % 60))
      else
        printf '{"text":""}\n'
      fi
      exit 0
    fi

    # recorder running with no state file is left over from a failed
    # stop; clear it so this run starts a new recording
    if running && [ ! -f "$state" ]; then
      sig -KILL
      sleep 0.2
    fi

    # ── stop ──
    if running; then
      # SIGINT so the recorder finishes writing the file
      sig -INT
      i=0
      while running && [ "$i" -lt 50 ]; do i=$((i + 1)); sleep 0.1; done
      if running; then
        sig -KILL
        ${pkgs.libnotify}/bin/notify-send -u critical "recording" "recorder did not stop, killed it"
      fi

      file=$(cat "$state" 2>/dev/null)
      rm -f "$state"
      bar

      if [ -z "$file" ] || [ ! -s "$file" ]; then
        ${pkgs.libnotify}/bin/notify-send -u critical "recording" "no file produced"
        exit 1
      fi

      size=$(du -h "$file" | cut -f1)
      MENU="${pkgs.wmenu}/bin/wmenu -f 'Inter 14' -N ${c.bg} -n ${c.fg} -S ${c.accent} -s ${c.bg} -M ${c.accent} -m ${c.bg} -l 5 -p 'rec ($size):'"
      choice=$(printf 'upload (zipline)\ncopy path\nopen\nsave only\ndelete' | eval "$MENU")

      case "$choice" in
        "upload (zipline)") "$HOME/.bin/zipline-upload" "$file" ;;
        "copy path")        printf '%s' "$file" | ${pkgs.wl-clipboard}/bin/wl-copy ;;
        open)               ${pkgs.xdg-utils}/bin/xdg-open "$file" >/dev/null 2>&1 & ;;
        "save only")        ${pkgs.libnotify}/bin/notify-send -c osd "recording" "saved to $file" ;;
        delete)             rm -f "$file" ;;
        *)                  : ;;
      esac
      exit 0
    fi

    # ── start ──
    vids=$(${pkgs.xdg-user-dirs}/bin/xdg-user-dir VIDEOS 2>/dev/null || echo "$HOME/Videos")
    dir="$vids/Recordings/$(date +%Y-%m)"
    mkdir -p "$dir"
    file="$dir/$(date +%Y-%m-%d_%H-%M-%S).mp4"

    # -o: click a monitor to record all of it, or drag a region
    geom=$(${pkgs.slurp}/bin/slurp -d -o 2>/dev/null) || exit 0
    [ -n "$geom" ] || exit 0

    # monitor of the default sink = desktop audio without the mic
    sink=$(${pkgs.pulseaudio}/bin/pactl get-default-sink 2>/dev/null)
    if [ -z "$sink" ]; then
      ${pkgs.libnotify}/bin/notify-send -u critical "recording" "no audio output found"
      exit 1
    fi

    set -- -g "$geom" --audio --audio-device "$sink.monitor" -f "$file"
    printf '%s' "$file" > "$state"

    start() {
      ${pkgs.util-linux}/bin/setsid -f ${pkgs.wl-screenrec}/bin/wl-screenrec "$@" >/dev/null 2>&1
      sleep 0.7
      running
    }

    if ! start "$@"; then
      # GPU encoding failed, try the CPU encoder
      rm -f "$file"
      if ! start --no-hw "$@"; then
        rm -f "$state" "$file"
        ${pkgs.libnotify}/bin/notify-send -u critical "recording" "wl-screenrec failed to start"
        exit 1
      fi
      ${pkgs.libnotify}/bin/notify-send -c osd "recording" "using software encoding"
    fi
    bar
  '';

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

  # ── Super+Shift+Tab: list every window in fuzzel, grouped by workspace.
  # Enter goes to it (or to the workspace, on a header), Shift+Enter opens
  # an action menu (bring here, float, scratchpad, close, force kill).
  # Source: ../pkgs/window-picker/main.go
  # fuzzel 1.14 has no --override, so the picker gets its own config: your
  # fuzzel settings with a monospace font (so the columns line up) and
  # Shift+Enter moved from execute-input to custom-1.
  pickerFuzzel = pkgs.writeText "window-picker-fuzzel.ini"
    (lib.generators.toINI { } (config.programs.fuzzel.settings // {
      main = config.programs.fuzzel.settings.main // {
        font = "MonaspiceAr Nerd Font:size=11";
      };
      key-bindings = {
        execute-input = "Control+Shift+Return";
        custom-1 = "Shift+Return";
      };
    }));

  windowPicker = pkgs.buildGoModule {
    pname = "window-picker";
    version = "2.6";
    src = ../pkgs/window-picker;
    vendorHash = null;   # standard library only
    ldflags = [
      "-s" "-w"
      "-X main.fuzzel=${pkgs.fuzzel}/bin/fuzzel"
      "-X main.config=${pickerFuzzel}"
    ];
    meta.mainProgram = "window-picker";
  };

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

  # ── app → workspace placement, toggled with Super+Alt+p ──
  # The rules live in ~/.config/sway/placement-on.conf. Sway includes
  # ~/.local/state/sway/placement.conf, a symlink to that file or to the
  # empty placement-off.conf; placement-toggle flips it and reloads sway.
  # Matches are exact (so kitty doesn't also catch kitty-float).
  # Check app ids with: swaymsg -t get_tree | grep -E 'app_id|class'
  # (app_id for wayland apps, class for xwayland)
  placementRules = {
    "1" = [ { app_id = "firefox"; } { app_id = "brave-browser"; } { class = "Brave-browser"; } { app_id = "floorp"; } ];
    "2" = [ { app_id = "spotify"; } { app_id = "signal"; } { app_id = "vesktop"; } ];
    "3" = [ { app_id = "mpv"; } ];
    "4" = [ { app_id = "kitty"; } ];
    "5" = [ { app_id = "thunar"; } ];
    "6" = [ { app_id = "org.qbittorrent.qBittorrent"; } ];
    "7" = [ { app_id = "virt-manager"; } ];
  };
  placementConf = lib.concatStrings (lib.flatten (lib.mapAttrsToList (ws: rules:
    map (r: "assign [${lib.concatStringsSep " "
      (lib.mapAttrsToList (k: v: "${k}=\"^${v}$\"") r)}] workspace number ${ws}\n") rules)
    placementRules));

  placementToggle = pkgs.writeShellApplication {
    name = "placement-toggle";
    runtimeInputs = [ pkgs.coreutils pkgs.sway pkgs.libnotify ];
    text = ''
      link="$HOME/.local/state/sway/placement.conf"
      cfg="$HOME/.config/sway"
      mkdir -p "$(dirname "$link")"
      if [ "$(readlink "$link" || true)" = "$cfg/placement-off.conf" ]; then
        state=on
      else
        state=off
      fi
      ln -sfn "$cfg/placement-$state.conf" "$link"
      swaymsg -q reload
      notify-send -c osd -h string:x-canonical-private-synchronous:osd \
        -t 1500 "window placement" "$state"
    '';
  };

  # switch to a workspace only while placement is on (Super+Return, Super+e)
  placementFollow = pkgs.writeShellApplication {
    name = "placement-follow";
    runtimeInputs = [ pkgs.coreutils pkgs.sway ];
    text = ''
      [ "$(readlink "$HOME/.local/state/sway/placement.conf" || true)" = "$HOME/.config/sway/placement-off.conf" ] && exit 0
      swaymsg -q "workspace number $1"
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
  home.packages = [ osd screenshot-menu screenshot-capture screenrec dnd ];

  xdg.configFile."sway/placement-on.conf".text = placementConf;
  xdg.configFile."sway/placement-off.conf".text = "# window placement off\n";
  # placement starts on; the toggle keeps its choice across rebuilds
  home.activation.swayPlacement = lib.hm.dag.entryAfter [ "linkGeneration" ] ''
    mkdir -p "$HOME/.local/state/sway"
    if [ ! -L "$HOME/.local/state/sway/placement.conf" ]; then
      ln -sfn "$HOME/.config/sway/placement-on.conf" "$HOME/.local/state/sway/placement.conf"
    fi
  '';

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
        "${mod}+semicolon" = "exec ${lib.getExe emojiPicker}";
        "${mod}+Shift+x" = "exec swaylock";
        "${mod}+Return" = "exec kitty; exec ${lib.getExe placementFollow} 4";
        "${mod}+e" = "exec thunar; exec ${lib.getExe placementFollow} 5";

        # window management
        "${mod}+q" = "kill";
        "${mod}+Shift+c" = "reload";
        "${mod}+Shift+e" = "exec swaynag -t warning -m 'exit sway?' -B 'yes' 'swaymsg exit'";
        "${mod}+f" = "exec ${lib.getExe fsToggle}";
        "${mod}+v" = "floating toggle";
        "${mod}+s" = "split toggle";

        # tabbed / stacked containers (replaces hyprland groups)
        "${mod}+g" = "exec ${lib.getExe cycleGroup} group";
        "${mod}+Shift+g" = "layout toggle stacking tabbed";
        "${mod}+Alt+p" = "exec ${lib.getExe placementToggle}";
        "${mod}+t" = "layout toggle split";
        "${mod}+Tab" = "exec ${lib.getExe cycleGroup} 1";
        "${mod}+Shift+Tab" = "exec ${lib.getExe windowPicker}";

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
        "${mod}+a" = "exec ${lib.getExe audioMenu}";
        "Shift+Print" = "exec screenrec";

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
      # no idle lock while a window is fullscreen
      for_window [all] inhibit_idle fullscreen
      # app -> workspace rules, toggled with Super+Alt+p
      include $HOME/.local/state/sway/placement.conf
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
