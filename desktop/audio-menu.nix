# Super+a: audio menu. Enter switches output/mic (current mic: mute),
# Shift+Enter hides or renames. Settings in ~/.config/uwuaudio.
{ config, pkgs, lib, ... }:
let
  audioMenu = pkgs.writeShellApplication {
    name = "audio-menu";
    runtimeInputs = [ pkgs.pulseaudio pkgs.jq pkgs.fuzzel pkgs.coreutils pkgs.gnugrep pkgs.gawk ];
    text = ''
      cfg="''${XDG_CONFIG_HOME:-$HOME/.config}/uwuaudio"
      hidden="$cfg/hidden" names="$cfg/names"
      mkdir -p "$cfg"; touch "$hidden"

      # names file: "device.name = Nickname"
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
      valid() { [[ $1 =~ ^[0-9]+$ ]] && (( $1 < $2 )); }
      menu() {
        fuzzel --dmenu --index --config ${config.my.pickerFuzzel} --width 60 \
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
            # move streams that were on the old default
            old=$(jq -r --arg n "$sink" '.[] | select(.name == $n) | .index' <<< "$sinks")
            pactl set-default-sink "$name"
            pactl list short sink-inputs | while read -r id on _; do
              if [ "$on" = "$old" ]; then pactl move-sink-input "$id" "$name" || true; fi
            done ;;
          source)
            # same for recording streams on the old mic
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
in
{
  wayland.windowManager.sway.config.keybindings."Mod4+a" = "exec ${lib.getExe audioMenu}";
}
