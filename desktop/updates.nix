# A bar block once nixpkgs (flake.lock) is two weeks old. Click: nxupdate
# in a popup, right-click: hide until next login.
{ pkgs, lib, ... }:
let
  updates = pkgs.writeShellApplication {
    name = "updates";
    runtimeInputs = with pkgs; [ coreutils jq procps ];
    text = ''
      # days old before it shows
      AFTER=14
      lock=/etc/nixos/flake.lock
      hidden="''${XDG_RUNTIME_DIR:-/tmp}/updates-hidden"

      case "''${1:-status}" in
        status)
          at=$(jq -r '.nodes.nixpkgs.locked.lastModified // empty' "$lock" 2>/dev/null || true)
          if [ -z "$at" ]; then
            echo '{"text":""}'
            exit 0
          fi
          days=$(( ($(date +%s) - at) / 86400 ))
          # hidden for this lock (a newer one shows again)
          if [ "$days" -lt "$AFTER" ] || { [ -f "$hidden" ] && [ "$(cat "$hidden")" = "$at" ]; }; then
            echo '{"text":""}'
            exit 0
          fi
          tip="nixpkgs is $days days old ($(date -d "@$at" '+%a %-d %b'))"$'\n'"click: nxupdate · right-click: hide until next login"
          jq -cn --arg t "󰚰  ''${days}d" --arg tip "$tip" '{text: $t, tooltip: $tip, class: ["msqu", "blue"]}'
          ;;
        hide)
          jq -r '.nodes.nixpkgs.locked.lastModified // empty' "$lock" > "$hidden"
          pkill -RTMIN+11 waybar || true
          ;;
        *)
          echo "usage: updates [status|hide]" >&2
          exit 1
          ;;
      esac
    '';
  };
in
{
  home.packages = [ updates ];

  # after the birthday block
  programs.waybar.settings.mainBar.modules-right = lib.mkOrder 650 [ "custom/updates" ];
  programs.waybar.settings.mainBar."custom/updates" = {
    return-type = "json";
    exec = "${lib.getExe updates} status";
    interval = 600;
    signal = 11;
    on-click = "foot --app-id foot-float zsh -ic \"nxupdate; echo; read -sk '?press a key to close'\"";
    on-click-right = "${lib.getExe updates} hide";
    exec-on-event = false;
  };
}
