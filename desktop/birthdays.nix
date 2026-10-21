# Birthdays in waybar: a pink block like the msqu ones (desktop/msqu.nix), shown
# from a few days before someone's birthday through the day; hovering it
# shows who. Left-click lists
# the next ones coming up; right-click hides it until you log in again
# (it comes back early only if another birthday comes into range).
#
# The names never go in this repo or the Nix store: the script reads them
# when it runs, from ~/.config/birthdays, one per line (# for comments):
#
#   14/03 Sam
#   02/11/1964 Mum        a year shows their age
#   7/8/98 Alex           dd/mm, dd/mm/yy or dd/mm/yyyy
#
# Without that file the cake never shows. It can be a link to a file kept
# elsewhere (on the pc, the Windows share); Windows line endings are fine.
{ pkgs, lib, osConfig, ... }:
let
  c = osConfig.theme.colors;

  birthday = pkgs.writeShellApplication {
    name = "birthday";
    runtimeInputs = with pkgs; [ coreutils gnused gnugrep gawk jq libnotify procps ];
    text = ''
      # days before a birthday the cake starts showing
      AHEAD=3
      file="''${XDG_CONFIG_HOME:-$HOME/.config}/birthdays"
      hidden="''${XDG_RUNTIME_DIR:-/tmp}/birthday-hidden"

      # escaped for waybar's tooltip (pango markup)
      esc() { sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' <<<"$1"; }
      # in UTC, so a day is always 86400s (no clock change in between)
      noon() { date -u -d "$1 12:00" +%s; }

      today=$(date +%F)
      now=$(noon "$today")
      year=$(date +%Y)

      # every birthday into $all, "days-until<TAB>date<TAB>name<TAB>age",
      # soonest first; lines it can't read into $bad
      all="" bad=""
      load() {
        [ -f "$file" ] || return 0
        local n=0 line d m y rest when on days age rows=()
        # Windows line endings and a Notepad BOM are fine
        while IFS= read -r line || [ -n "$line" ]; do
          n=$((n + 1))
          line=''${line%$'\r'}
          [ "$n" = 1 ] && line=''${line#$'\xef\xbb\xbf'}
          # trimmed in bash: no extra process for every line
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
          # check it's a real day (29/02 is, in a leap year)
          if ! date -d "2000-$m-$d" >/dev/null 2>&1 || [ "$m" -gt 12 ]; then
            bad+="''${bad:+, }$n"
            continue
          fi
          # the next one, today included; 29/02 is the 28th in other years
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
          # (not for a baby's first: "turns 0")
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
          # hidden: until one comes into range that wasn't showing then
          if [ -z "$upcoming" ] || { [ -f "$hidden" ] && ! cut -f2,3 <<<"$upcoming" | grep -qvxFf "$hidden"; }; then
            echo '{"text":""}'
            exit 0
          fi
          first=$(head -n1 <<<"$upcoming" | cut -f1)
          # nerd font cake, as the msqu blocks use; two spaces, since the
          # icon is wider than the space its font gives it and eats one
          text="󰃫  today"
          [ "$first" -gt 0 ] && text="󰃫  ''${first}d"
          tip=""
          while IFS=$'\t' read -r days on name age; do
            tip+="$(esc "$(describe "$days" "$on" "$name" "$age")")"$'\n'
          done <<<"$upcoming"
          tip+="<span color='#${c.muted}'>right-click to hide until next login</span>"
          # msqu's magenta block, one colour throughout (icon and text);
          # on the day a stronger tint rather than a second colour
          day=soon
          [ "$first" = 0 ] && day=today
          jq -cn --arg t "$text" --arg tip "$tip" --arg d "$day" \
            '{text: $t, tooltip: $tip, class: ["msqu", "magenta", $d]}'
          ;;
        hide)
          # remembers what was showing; only a new one shows it again
          soon | cut -f2,3 >> "$hidden"
          pkill -RTMIN+11 waybar || true
          ;;
        next)
          out=""
          while IFS=$'\t' read -r days on name age; do
            # escaped: mako reads markup too
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

  # right after the msqu blocks (they're lib.mkBefore, order 500; the rest
  # of the bar is the default, 1000)
  programs.waybar.settings.mainBar.modules-right = lib.mkOrder 600 [ "custom/birthday" ];
  programs.waybar.settings.mainBar."custom/birthday" = {
    return-type = "json";
    exec = "${lib.getExe birthday} status";
    # often enough to notice the day changing
    interval = 600;
    signal = 11;
    on-click = "${lib.getExe birthday} next";
    on-click-right = "${lib.getExe birthday} hide";
    exec-on-event = false;
  };

  # on the day: the same pink on a stronger tint (no line along the
  # bottom: on the msqu blocks that means paused)
  programs.waybar.style = lib.mkAfter ''
    .msqu.magenta.today { background: alpha(#${c.magenta}, 0.28); }
  '';
}
