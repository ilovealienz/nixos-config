{ pkgs, ... }:
{
  programs.waybar.settings.mainBar."custom/weather" = {
    format = "{}";
    return-type = "json";
    interval = 300;
    exec = "${pkgs.writeShellScript "weather" ''
      # ── settings ──────────────────────────────────
      LAT=53.8179442
      LON=-3.0509812
      UNITS=metric          # metric | imperial | standard
      DEGREE="°C"           # °C | °F | K  (match UNITS)
      SEP="   "              # spacing between icon and temp
      WIND_UNIT="m/s"       # m/s for metric, mph for imperial
      CACHE_AGE=1800        # seconds before refetching
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
      temp=$($JQ -r '.main.temp | round' "$CACHE")
      feels=$($JQ -r '.main.feels_like | round' "$CACHE")
      desc=$($JQ -r '.weather[0].description' "$CACHE")
      hum=$($JQ -r '.main.humidity' "$CACHE")
      wind=$($JQ -r '.wind.speed | round' "$CACHE")

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

      printf '{"text":"%s%s%s%s","tooltip":"%s\\nfeels like %s%s\\nhumidity %s%%\\nwind %s %s","class":"%s"}\n' \
        "$icon" "$SEP" "$temp" "$DEGREE" \
        "$desc" "$feels" "$DEGREE" "$hum" "$wind" "$WIND_UNIT" \
        "$class"
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
