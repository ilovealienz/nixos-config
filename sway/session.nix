# Session menu on Super+Shift+E (fuzzel): shut down, reboot, log out,
# suspend, lock. Shut down, reboot and log out ask "are you sure" with yes
# first, so Enter twice shuts down; Escape cancels at either step.
{ pkgs, ... }:
let
  sessionMenu = pkgs.writeShellApplication {
    name = "session-menu";
    runtimeInputs = with pkgs; [ fuzzel sway systemd swaylock libnotify ];
    text = ''
      # suspend, reboot and shut down can be refused (another user logged
      # in, or a program holding an inhibitor); say why instead of nothing
      power() {
        local err
        if ! err=$(systemctl "$1" 2>&1); then
          notify-send -u critical "session" "$2 refused: $err"
        fi
      }

      # yes first, so Enter twice does it; Escape or "no" cancels
      confirm() {
        local pick
        pick=$(printf '%s\n' "yes, $1" "no" | fuzzel --dmenu --index --prompt "$1? " \
                 --lines 2 --width 24) || return 1
        [ "$pick" = 0 ]
      }

      rows=("shut down" "reboot" "log out" "suspend" "lock")
      i=$(printf '%s\n' "''${rows[@]}" | fuzzel --dmenu --index --prompt 'session: ' \
            --lines "''${#rows[@]}" --width 24) || exit 0
      case "$i" in
        0) confirm "shut down" && power poweroff "shut down" ;;
        1) confirm "reboot" && power reboot "reboot" ;;
        # home-manager's sway setup stops sway-session.target once sway exits
        2) confirm "log out" && swaymsg exit ;;
        3) power suspend "suspend" ;;      # swayidle locks before sleep
        4) swaylock -f ;;
      esac
    '';
  };
in
{
  # on PATH for the Super+Shift+E binding in compositor.nix
  home.packages = [ sessionMenu ];
}
