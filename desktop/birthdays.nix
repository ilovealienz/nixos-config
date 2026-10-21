# Birthdays in waybar, shown a few days ahead. Click: upcoming, right-click:
# hide until next login. Read at runtime from ~/.config/birthdays:
#
#   14/03 Sam
#   02/11/1964 Mum        a year shows their age
#   7/8/98 Alex           dd/mm, dd/mm/yy or dd/mm/yyyy
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  birthday = pkgs.writeShellApplication {
    name = "birthday";
    runtimeInputs = with pkgs; [ coreutils gnused gnugrep gawk jq libnotify procps ];
    text = ''
      # days ahead to show
      AHEAD=3
      file="''${XDG_CONFIG_HOME:-$HOME/.config}/birthdays"
      hidden="''${XDG_RUNTIME_DIR:-/tmp}/birthday-hidden"

      # escape pango markup
      esc() { sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' <<<"$1"; }
      # UTC, so a day is always 86400s
      noon() { date -u -d "$1 12:00" +%s; }

      today=$(date +%F)
      now=$(noon "$today")
      year=$(date +%Y)

      # $all: "days<TAB>date<TAB>name<TAB>age", soonest first; $bad: unreadable lines
      all="" bad=""
      load() {
        [ -f "$file" ] || return 0
        local n=0 line d m y rest when on days age rows=()
        # handles CRLF and a BOM
        while IFS= read -r line || [ -n "$line" ]; do
          n=$((n + 1))
          line=''${line%$'\r'}
          [ "$n" = 1 ] && line=''${line#$'\xef\xbb\xbf'}
          # trim
          line=''${line#"''${line%%[![:space:]]*}"}
          line=''${line%"''${line##*[![:space:]]}"}
          case "$line" in "" | "#"*) continue ;; esac
          if [[ "$line" =~ ^([0-9]{1,2})/([0-9]{1,2})(/([0-9]{2}|[0-9]{4}))?[[:space:]]+(.+)$ ]]; then
            d=$((10#''${BASH_REMATCH[1]})) m=$((10#''${BASH_REMATCH[2]}))
            y=''${BASH_REMATCH[4]} rest=''${BASH_REMATCH[5]}
          else
            bad+="''${bad:+, }$n"
            continue
          fi
          # dd/mm/yy: this century unless that would be in the future
          if [ "''${#y}" = 2 ]; then
            if [ $((10#$y)) -le $((10#''${year:2:2})) ]; then y=$((2000 + 10#$y)); else y=$((1900 + 10#$y)); fi
          fi
          # a real day
          if ! date -d "2000-$m-$d" >/dev/null 2>&1 || [ "$m" -gt 12 ]; then
            bad+="''${bad:+, }$n"
            continue
          fi
          # next one, today included; 29/02 is the 28th in other years
          for when in "$year" $((year + 1)); do
            if date -d "$when-$m-$d" >/dev/null 2>&1; then
              on=$(date -d "$when-$m-$d" +%F)
            else
              on=$(date -d "$when-02-28" +%F)
            fi
            days=$(( ($(noon "$on") - now) / 86400 ))
            [ "$days" -ge 0 ] && break
          done
          age=""
          [ -n "$y" ] && [ $(( ''${on:0:4} - y )) -gt 0 ] && age=$(( ''${on:0:4} - y ))
          rows+=("$days"$'\t'"$on"$'\t'"$rest"$'\t'"$age")
        done < "$file"
        [ "''${#rows[@]}" -gt 0 ] && all=$(printf '%s\n' "''${rows[@]}" | sort -n -k1,1)
        return 0
      }
      load

      # the ones close enough to show
      soon() { [ -n "$all" ] && awk -F'\t' -v a="$AHEAD" '$1 <= a' <<<"$all"; return 0; }

      describe() {
        local days=$1 on=$2 name=$3 age=$4 when
        case "$days" in
          0) when="today" ;;
          1) when="tomorrow" ;;
          *) when="$(date -d "$on" '+%a %-d %b'), in $days days" ;;
        esac
        printf '%s, %s%s' "$name" "$when" "''${age:+ (turns $age)}"
      }

      case "''${1:-status}" in
        status)
          upcoming=$(soon)
          # hidden until a new one comes into range
          if [ -z "$upcoming" ] || { [ -f "$hidden" ] && ! cut -f2,3 <<<"$upcoming" | grep -qvxFf "$hidden"; }; then
            echo '{"text":""}'
            exit 0
          fi
          first=$(head -n1 <<<"$upcoming" | cut -f1)
          # two spaces: the icon overlaps one
          text="󰃫  today"
          [ "$first" -gt 0 ] && text="󰃫  ''${first}d"
          tip=""
          while IFS=$'\t' read -r days on name age; do
            tip+="$(esc "$(describe "$days" "$on" "$name" "$age")")"$'\n'
          done <<<"$upcoming"
          tip+="<span color='#${c.muted}'>right-click to hide until next login</span>"
          day=soon
          [ "$first" = 0 ] && day=today
          jq -cn --arg t "$text" --arg tip "$tip" --arg d "$day" \
            '{text: $t, tooltip: $tip, class: ["msqu", "magenta", $d]}'
          ;;
        hide)
          soon | cut -f2,3 >> "$hidden"
          pkill -RTMIN+11 waybar || true
          ;;
        next)
          out=""
          while IFS=$'\t' read -r days on name age; do
            [ -n "$days" ] && out+="$(esc "$(describe "$days" "$on" "$name" "$age")")"$'\n'
          done < <([ -n "$all" ] && head -n 5 <<<"$all")
          if [ -n "$bad" ]; then
            case "$bad" in *,*) out+="couldn't read lines $bad of $(esc "$file")" ;; *) out+="couldn't read line $bad of $(esc "$file")" ;; esac
          fi
          [ -n "$out" ] || out="no birthdays in $(esc "$file")"
          notify-send -a birthdays "coming up" "$out"
          ;;
        *)
          echo "usage: birthday [status|hide|next]" >&2
          exit 1
          ;;
      esac
    '';
  };
in
{
  home.packages = [ birthday ];

  # right after the msqu blocks
  programs.waybar.settings.mainBar.modules-right = lib.mkOrder 600 [ "custom/birthday" ];
  programs.waybar.settings.mainBar."custom/birthday" = {
    return-type = "json";
    exec = "${lib.getExe birthday} status";
    interval = 600;
    signal = 11;
    on-click = "${lib.getExe birthday} next";
    on-click-right = "${lib.getExe birthday} hide";
    exec-on-event = false;
  };

  # on the day: a stronger tint
  programs.waybar.style = lib.mkAfter ''
    .msqu.magenta.today { background: alpha(#${c.magenta}, 0.28); }
  '';
}
