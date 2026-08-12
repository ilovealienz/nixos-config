{ pkgs, ... }:
{
  programs.waybar.settings.mainBar."custom/weather" = {
    format = "{}";
    return-type = "json";
    interval = 300;
    tooltip = true;
    markup = "pango";
    escape = false;
    exec = "${pkgs.writeShellScript "weather" ''
      # ── settings ──────────────────────────────────
      LAT=53.8179442
      LON=-3.0509812
      UNITS=metric          # metric | imperial | standard
      DEGREE="°C"           # °C | °F | K  (match UNITS)
      SEP="   "             # spacing between icon and temp
      WIND_UNIT="m/s"       # m/s for metric, mph for imperial
      CACHE_AGE=1800        # seconds before refetching

      # tooltip colours
      C_HEAD="#e5a440"      # condition heading
      C_LABEL="#87765d"     # muted labels
      C_VALUE="#d4b07b"     # values
      # ──────────────────────────────────────────────

      CACHE="$HOME/.cache/weather.json"
      KEYFILE="$HOME/.config/weather/key"

      [ -f "$KEYFILE" ] || { echo '{"text":"","tooltip":"no api key"}'; exit 0; }
      KEY=$(cat "$KEYFILE")
      mkdir -p "$(dirname "$CACHE")"

      if [ ! -f "$CACHE" ] || [ "$(( $(date +%s) - $(stat -c %Y "$CACHE") ))" -gt "$CACHE_AGE" ]; then
        ${pkgs.curl}/bin/curl -sf --max-time 10 \
          "https://api.openweathermap.org/data/2.5/weather?lat=$LAT&lon=$LON&appid=$KEY&units=$UNITS" \
          -o "$CACHE.tmp" && mv "$CACHE.tmp" "$CACHE"
      fi

      [ -f "$CACHE" ] || { echo '{"text":"","tooltip":"unavailable"}'; exit 0; }

      JQ=${pkgs.jq}/bin/jq
      code=$($JQ -r '.weather[0].icon' "$CACHE")
      desc=$($JQ -r '.weather[0].description' "$CACHE")
      temp=$($JQ -r '.main.temp | round' "$CACHE")
      feels=$($JQ -r '.main.feels_like | round' "$CACHE")
      tmin=$($JQ -r '.main.temp_min | round' "$CACHE")
      tmax=$($JQ -r '.main.temp_max | round' "$CACHE")
      hum=$($JQ -r '.main.humidity' "$CACHE")
      press=$($JQ -r '.main.pressure' "$CACHE")
      wind=$($JQ -r '.wind.speed | round' "$CACHE")
      wdeg=$($JQ -r '.wind.deg' "$CACHE")
      clouds=$($JQ -r '.clouds.all' "$CACHE")
      vis=$(( $($JQ -r '.visibility' "$CACHE") / 1000 ))
      city=$($JQ -r '.name' "$CACHE")
      sunrise=$(date -d "@$($JQ -r '.sys.sunrise' "$CACHE")" +%H:%M)
      sunset=$(date -d "@$($JQ -r '.sys.sunset' "$CACHE")" +%H:%M)

      # wind degrees → compass point
      dirs=(N NNE NE ENE E ESE SE SSE S SSW SW WSW W WNW NW NNW N)
      wdir=''${dirs[$(( (wdeg + 11) / 22 ))]}

      case "$code" in
        01d) icon="󰖙"; class="clear"   ;;
        01n) icon="󰖔"; class="clear"   ;;
        02d) icon="󰖕"; class="cloudy"  ;;
        02n) icon="󰼱"; class="cloudy"  ;;
        03*|04*) icon="󰖐"; class="cloudy" ;;
        09*) icon="󰖗"; class="rainy"   ;;
        10*) icon="󰖖"; class="rainy"   ;;
        11*) icon="󰖓"; class="stormy"  ;;
        13*) icon="󰖘"; class="snowy"   ;;
        50*) icon="󰖑"; class="foggy"   ;;
        *)   icon="󰼯"; class="unknown" ;;
      esac

      tip="<span size='large' color='$C_HEAD'><b>$icon  $desc</b></span>
<span color='$C_LABEL'>$city  ·  $temp$DEGREE  (feels $feels$DEGREE)</span>

<span color='$C_LABEL'>high / low </span><span color='$C_VALUE'>$tmax$DEGREE / $tmin$DEGREE</span>
<span color='$C_LABEL'>humidity   </span><span color='$C_VALUE'>$hum%</span>
<span color='$C_LABEL'>wind       </span><span color='$C_VALUE'>$wind $WIND_UNIT  $wdir</span>
<span color='$C_LABEL'>pressure   </span><span color='$C_VALUE'>$press hPa</span>
<span color='$C_LABEL'>cloud      </span><span color='$C_VALUE'>$clouds%</span>
<span color='$C_LABEL'>visibility </span><span color='$C_VALUE'>$vis km</span>

<span color='$C_LABEL'>󰖜 </span><span color='$C_VALUE'>$sunrise</span>   <span color='$C_LABEL'>󰖛 </span><span color='$C_VALUE'>$sunset</span>"

      tipjson=$(printf '%s' "$tip" | $JQ -Rs .)

      printf '{"text":"%s%s%s%s","tooltip":%s,"class":"%s"}\n' \
        "$icon" "$SEP" "$temp" "$DEGREE" "$tipjson" "$class"
    ''}";
  };

  programs.waybar.style = ''
    #custom-weather { padding: 0 9px; color: #d4b07b; }
    #custom-weather.clear  { color: #e5a440; }
    #custom-weather.rainy  { color: #949fb4; }
    #custom-weather.stormy { color: #e56b55; }
    #custom-weather.snowy  { color: #bfab36; }
  '';
}
