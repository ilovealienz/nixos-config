# Screen recording, Shift+Print (or click the bar's REC block to stop),
# with the REC block in waybar.
{ config, pkgs, osConfig, ... }:
let
  c = osConfig.theme.colors;

  # Toggle: first run starts, second run (or clicking the bar module)
  # stops. Records desktop audio only, via the default sink's monitor.
  # wl-screenrec encodes on the GPU (VAAPI) and falls back to a software
  # encoder when that fails, e.g. no VAAPI driver.
  # Recording process name, used for pgrep/pkill:
  #   wl-screenrec (current) and wf-recorder (older versions of this script)
  screenrec = pkgs.writeShellScriptBin "screenrec" ''
    run="''${XDG_RUNTIME_DIR:-/tmp}"
    state="$run/screenrec.state"   # path of the file being recorded
    busy="$run/screenrec.busy"     # "saving" or "uploading" while finishing
    lock="$run/screenrec.lock"
    running() { ${pkgs.procps}/bin/pgrep -x 'wl-screenrec|wf-recorder' >/dev/null; }
    sig() { ${pkgs.procps}/bin/pkill "$1" -x 'wl-screenrec|wf-recorder'; }
    bar() { ${pkgs.procps}/bin/pkill -RTMIN+8 waybar; }
    # 9>&-: nothing started from here may keep the lock (fd 9) open after
    # this script exits; wl-copy, for one, stays running in the background
    notify() { ${pkgs.libnotify}/bin/notify-send "$@" 9>&-; }

    # ── waybar module ──
    if [ "''${1:-}" = status ]; then
      # a busy file with nobody holding the lock is left over from a run
      # that was killed; ignore it
      if [ -f "$busy" ] && ! ${pkgs.util-linux}/bin/flock -n -s "$lock" true 2>/dev/null; then
        case "$(cat "$busy" 2>/dev/null)" in
          uploading) printf '{"text":"⬆ uploading","class":"busy","tooltip":"uploading the recording"}\n' ;;
          *)         printf '{"text":"● saving","class":"busy","tooltip":"finishing the recording file"}\n' ;;
        esac
      elif running && [ -f "$state" ]; then
        s=$(( $(date +%s) - $(stat -c %Y "$state") ))
        printf '{"text":"● REC %02d:%02d","class":"recording","tooltip":"click to stop"}\n' \
          $((s / 60)) $((s % 60))
      else
        printf '{"text":""}\n'
      fi
      exit 0
    fi

    # run in our own session: waybar kills what its clicks started when it
    # restarts (a sway reload does that), which would cut off a save or upload
    if [ -z "''${SCREENREC_DETACHED:-}" ]; then
      SCREENREC_DETACHED=1 exec ${pkgs.util-linux}/bin/setsid -f "$0" "$@"
    fi

    # one screenrec at a time: a click while a recording is being saved,
    # picked from the menu or uploaded only shows a notification, instead
    # of starting a region selection on top of it
    # (-w 0.5: the bar's once-a-second status check holds it for a moment)
    exec 9>"$lock"
    if ! ${pkgs.util-linux}/bin/flock -w 0.5 9; then
      notify -c osd "recording" "still finishing the last recording, wait a moment"
      exit 0
    fi
    trap 'rm -f "$busy"; bar' EXIT

    # recorder running with no state file is left over from a failed
    # stop; clear it so this run starts a new recording
    if running && [ ! -f "$state" ]; then
      sig -KILL
      sleep 0.2
    fi

    # ── stop ──
    if running; then
      echo saving > "$busy"; bar
      # SIGINT so the recorder finishes writing the file; a long recording
      # can take a while, and killing it early leaves a broken file
      sig -INT
      i=0
      while running && [ "$i" -lt 600 ]; do i=$((i + 1)); sleep 0.1; done
      if running; then
        sig -KILL
        notify -u critical "recording" "recorder did not stop after a minute, killed it"
      fi

      file=$(cat "$state" 2>/dev/null)
      rm -f "$state" "$busy"
      bar

      if [ -z "$file" ] || [ ! -s "$file" ]; then
        notify -u critical "recording" "no file produced"
        exit 1
      fi

      size=$(du -h "$file" | cut -f1)
      MENU="${pkgs.wmenu}/bin/wmenu -f 'Inter 14' ${config.my.wmenuColours} -l 5 -p 'rec ($size):'"
      choice=$(printf 'upload (zipline)\ncopy path\nopen\nsave only\ndelete' | eval "$MENU")

      case "$choice" in
        "upload (zipline)")
          echo uploading > "$busy"; bar
          if ! "$HOME/.bin/zipline-upload" "$file" 9>&-; then
            notify -u critical "recording" "upload failed, the file is still at $file"
          fi ;;
        "copy path") printf '%s' "$file" | ${pkgs.wl-clipboard}/bin/wl-copy 9>&- ;;
        open)        ${pkgs.xdg-utils}/bin/xdg-open "$file" >/dev/null 2>&1 9>&- & ;;
        "save only") notify -c osd "recording" "saved to $file" ;;
        delete)      rm -f "$file" ;;
        *)           : ;;
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
      notify -u critical "recording" "no audio output found"
      exit 1
    fi

    set -- -g "$geom" --audio --audio-device "$sink.monitor" -f "$file"
    printf '%s' "$file" > "$state"

    start() {
      # 9>&-: the recorder must not keep holding the lock, or stopping
      # would be refused for the whole recording
      ${pkgs.util-linux}/bin/setsid -f ${pkgs.wl-screenrec}/bin/wl-screenrec "$@" >/dev/null 2>&1 9>&-
      sleep 0.7
      running
    }

    if ! start "$@"; then
      # GPU encoding failed, try the CPU encoder
      rm -f "$file"
      if ! start --no-hw "$@"; then
        rm -f "$state" "$file"
        notify -u critical "recording" "wl-screenrec failed to start"
        exit 1
      fi
      notify -c osd "recording" "using software encoding"
    fi
  '';
in
{
  home.packages = [ screenrec ];

  wayland.windowManager.sway.config.keybindings."Shift+Print" = "exec screenrec";

  # where it sits in the bar: modules-right in waybar.nix
  programs.waybar.settings.mainBar."custom/rec" = {
    return-type = "json";
    interval = 1;
    signal = 8;
    exec = "screenrec status";
    on-click = "screenrec";
  };

  programs.waybar.style = ''
    #custom-rec {
      padding: 0 8px;
      color: #${c.onAccent};
      background: #${c.red};
    }
    #custom-rec.busy {
      background: #${c.orange};
    }
  '';
}
