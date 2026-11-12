# Birthdays and other yearly days in waybar, shown a few days ahead, with a
# notification on the day. Click: upcoming, right-click: hide until next
# login. Read at runtime from ~/.config/birthdays:
#
#   14/03 Sam
#   02/11/1964 Mum                 a year shows their age
#   7/8/98 Alex                    dd/mm, dd/mm/yy or dd/mm/yyyy
#   29/10/1998 Me | command        "| action": clicking the notification runs it
#   29/10/1998 Me | = command      "| =": its output is shown in the notification
#
#   [yearly]                       below this: events, not birthdays
#   25/12 Christmas
#   31/12 23:00 New year video | https://youtube.com/...
#
# The notification comes at the time given, or 09:00. An action that's a link
# opens in the browser; anything else runs in a terminal, with ~/.local/bin
# and ~/.bin on its PATH.
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  birthday = pkgs.writeShellApplication {
    name = "birthday";
    runtimeInputs = with pkgs; [ coreutils gnused gnugrep gawk jq libnotify procps systemd ];
    text = ''
      # days ahead to show
      AHEAD=3
      # untimed ones notify from this time on the day
      DEFAULT_TIME=09:00
      file="''${XDG_CONFIG_HOME:-$HOME/.config}/birthdays"
      hidden="''${XDG_RUNTIME_DIR:-/tmp}/birthday-hidden"
      fired="''${XDG_STATE_HOME:-$HOME/.local/state}/birthday-fired"

      # escape pango markup
      esc() { sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' <<<"$1"; }
      # UTC, so a day is always 86400s
      noon() { date -u -d "$1 12:00" +%s; }

      today=$(date +%F)
      now=$(noon "$today")
      year=$(date +%Y)

      # $all: days, date, name, age, kind, time, action, split by $FS (not tab: bash
      # merges empty tab-split fields), soonest first; $bad: unreadable lines
      FS=$'\x1f'
      all="" bad=""
      load() {
        [ -f "$file" ] || return 0
        local n=0 kind=birthday line d m y at rest name action when on days age rows=()
        local re='^([0-9]{1,2})/([0-9]{1,2})(/([0-9]{2}|[0-9]{4}))?[[:space:]]+(([0-9]{1,2}:[0-9]{2})[[:space:]]+)?(.+)$'
        # handles CRLF and a BOM
        while IFS= read -r line || [ -n "$line" ]; do
          n=$((n + 1))
          line=''${line%$'\r'}
          [ "$n" = 1 ] && line=''${line#$'\xef\xbb\xbf'}
          # trim
          line=''${line#"''${line%%[![:space:]]*}"}
          line=''${line%"''${line##*[![:space:]]}"}
          case "''${line,,}" in
            "" | "#"*) continue ;;
            "[birthdays]") kind=birthday; continue ;;
            "[yearly]") kind=yearly; continue ;;
          esac
          if [[ "$line" =~ $re ]]; then
            d=$((10#''${BASH_REMATCH[1]})) m=$((10#''${BASH_REMATCH[2]}))
            y=''${BASH_REMATCH[4]} at=''${BASH_REMATCH[6]} rest=''${BASH_REMATCH[7]}
          else
            bad+="''${bad:+, }$n"
            continue
          fi
          # "name | action"
          name=$rest action=""
          if [[ "$rest" == *" | "* ]]; then
            name=''${rest%% | *} action=''${rest#* | }
          fi
          # dd/mm/yy: this century unless that would be in the future
          if [ "''${#y}" = 2 ]; then
            if [ $((10#$y)) -le $((10#''${year:2:2})) ]; then y=$((2000 + 10#$y)); else y=$((1900 + 10#$y)); fi
          fi
          # a real day and time
          if ! date -d "2000-$m-$d" >/dev/null 2>&1 || [ "$m" -gt 12 ] ||
             { [ -n "$at" ] && ! date -d "2000-01-01 $at" >/dev/null 2>&1; }; then
            bad+="''${bad:+, }$n"
            continue
          fi
          [ -n "$at" ] && at=$(date -d "2000-01-01 $at" +%H:%M)
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
          rows+=("$days$FS$on$FS$name$FS$age$FS$kind$FS$at$FS$action")
        done < "$file"
        [ "''${#rows[@]}" -gt 0 ] && all=$(printf '%s\n' "''${rows[@]}" | sort -t"$FS" -k1,1n -k6,6)
        return 0
      }
      load

      # the ones close enough to show
      soon() { [ -n "$all" ] && awk -F"$FS" -v a="$AHEAD" '$1 <= a' <<<"$all"; return 0; }

      describe() {
        local days=$1 on=$2 name=$3 age=$4 kind=$5 at=$6 when extra=""
        case "$days" in
          0) when="today" ;;
          1) when="tomorrow" ;;
          *) when="$(date -d "$on" '+%a %-d %b'), in $days days" ;;
        esac
        [ -n "$at" ] && when+=" at $at"
        if [ -n "$age" ]; then
          if [ "$kind" = birthday ]; then extra=" (turns $age)"
          elif [ "$age" = 1 ]; then extra=" (1 year)"
          else extra=" ($age years)"
          fi
        fi
        printf '%s, %s%s' "$name" "$when" "$extra"
      }

      # a link opens in the browser, anything else runs in a terminal
      run() {
        case "$1" in
          http://* | https://*) PATH="/run/current-system/sw/bin:$PATH" xdg-open "$1" ;;
          *) PATH="$HOME/.local/bin:$HOME/.bin:/etc/profiles/per-user/$USER/bin:/run/current-system/sw/bin:$PATH" foot --app-id foot-float --hold sh -c "$1" ;;
        esac
      }

      case "''${1:-status}" in
        status)
          upcoming=$(soon)
          # hidden until a new one comes into range
          if [ -z "$upcoming" ] || { [ -f "$hidden" ] && ! cut -d"$FS" -f2,3 <<<"$upcoming" | grep -qvxFf "$hidden"; }; then
            echo '{"text":""}'
            exit 0
          fi
          first=$(head -n1 <<<"$upcoming" | cut -d"$FS" -f1)
          # two spaces: the icon overlaps one
          text="󰃫  today"
          [ "$first" -gt 0 ] && text="󰃫  ''${first}d"
          tip=""
          while IFS=$FS read -r days on name age kind at _; do
            tip+="$(esc "$(describe "$days" "$on" "$name" "$age" "$kind" "$at")")"$'\n'
          done <<<"$upcoming"
          tip+="<span color='#${c.muted}'>right-click to hide until next login</span>"
          day=soon
          [ "$first" = 0 ] && day=today
          jq -cn --arg t "$text" --arg tip "$tip" --arg d "$day" \
            '{text: $t, tooltip: $tip, class: ["msqu", "magenta", $d]}'
          ;;
        hide)
          soon | cut -d"$FS" -f2,3 >> "$hidden"
          pkill -RTMIN+11 waybar || true
          ;;
        next)
          out=""
          while IFS=$FS read -r days on name age kind at _; do
            [ -n "$days" ] && out+="$(esc "$(describe "$days" "$on" "$name" "$age" "$kind" "$at")")"$'\n'
          done < <([ -n "$all" ] && head -n 5 <<<"$all")
          if [ -n "$bad" ]; then
            case "$bad" in *,*) out+="couldn't read lines $bad of $(esc "$file")" ;; *) out+="couldn't read line $bad of $(esc "$file")" ;; esac
          fi
          [ -n "$out" ] || out="nothing in $(esc "$file")"
          notify-send -a birthdays "coming up" "$out"
          ;;
        due)
          # today's that have reached their time and haven't notified yet
          # no notification daemon (yet): leave them for the next check
          busctl --user call org.freedesktop.Notifications /org/freedesktop/Notifications \
            org.freedesktop.Notifications GetServerInformation >/dev/null 2>&1 || exit 0
          mkdir -p "$(dirname "$fired")"
          touch "$fired"
          # only today's are kept
          { grep "^$today"$'\t' "$fired" || true; } > "$fired.new"
          mv "$fired.new" "$fired"
          clock=$(date +%H:%M)
          while IFS=$FS read -r days on name age kind at action; do
            [ "$days" = 0 ] || continue
            [[ "''${at:-$DEFAULT_TIME}" > "$clock" ]] && continue
            grep -qxF "$today"$'\t'"$name" "$fired" && continue
            body=$(esc "$(describe "$days" "$on" "$name" "$age" "$kind" "$at")")
            # marked only once sent, so it's tried again if there's no notification daemon yet
            if [[ "$action" == "="* ]]; then
              # "= command": its output goes in the notification
              extra=$(sh -c "''${action#=}" 2>&1 | head -n 5 || true)
              notify-send -a birthdays "today" "$body''${extra:+$'\n'$(esc "$extra")}" || continue
            elif [ -n "$action" ]; then
              # waits for the click on its own (for a day at most), so this check can finish
              systemd-run --user --quiet --collect -p RuntimeMaxSec=1d -- "$0" notify "$body" "$action" || continue
            else
              notify-send -a birthdays "today" "$body" || continue
            fi
            printf '%s\t%s\n' "$today" "$name" >> "$fired"
          done < <([ -n "$all" ] && awk -F"$FS" '$1 == 0' <<<"$all")
          ;;
        notify)
          # stays until clicked or closed; a click runs the action
          picked=$(notify-send -a birthdays -t 0 -A default=open "today" "$2 (click to open)" || true)
          if [ "$picked" = default ]; then run "$3"; fi
          ;;
        *)
          echo "usage: birthday [status|hide|next|due]" >&2
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

  # notifications on the day
  systemd.user.services.birthday-due = {
    Unit.Description = "Notify about today's birthdays and yearly days";
    Service = {
      Type = "oneshot";
      ExecStart = "${lib.getExe birthday} due";
    };
  };
  systemd.user.timers.birthday-due = {
    Unit.Description = "Check for today's birthdays and yearly days";
    Timer = {
      OnStartupSec = "1min";
      OnCalendar = "*:0/5";
    };
    Install.WantedBy = [ "timers.target" ];
  };

  # on the day: a stronger tint
  programs.waybar.style = lib.mkAfter ''
    .msqu.magenta.today { background: alpha(#${c.magenta}, 0.28); }
  '';
}
