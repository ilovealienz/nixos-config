# The thin sliver at the right end of waybar:
#   left-click   stay awake (amber): no lock or screen-off
#   right-click  game mode (green): stay awake, and GameMode on for as long
#                as it's on (performance profile held, plus GameMode's own
#                tweaks), whatever you're running
# Click the same button again to turn it off. While one is on, the other
# button does nothing, so the two can't be mixed.
# Staying awake swaps swayidle for one that only locks before sleep and on
# `loginctl lock-session`, so closing the lid still locks.
{ pkgs, ... }:
let
  lock = "${pkgs.swaylock}/bin/swaylock -f";

  awake = pkgs.writeShellApplication {
    name = "awake";
    runtimeInputs = with pkgs; [ systemd procps util-linux libnotify ];
    text = ''
      # one click at a time: a quick left then right click would otherwise
      # both see "off" and end up mixing the two modes. The status check
      # only reads, so it doesn't wait for this
      lock() {
        exec 9>"''${XDG_RUNTIME_DIR:-/tmp}/awake.lock"
        flock 9
      }

      on() { systemctl --user is-active -q "$1"; }
      state() {
        if on awake-game.service; then echo game
        elif on awake.service; then echo awake
        else echo off
        fi
      }
      bar() { pkill -RTMIN+10 waybar || true; }

      case "''${1:-status}" in
        status)
          case $(state) in
            game)  printf '%s\n' '{"text":" ","class":"game","tooltip":"game mode: no lock, GameMode on\nright-click to turn off"}' ;;
            awake) printf '%s\n' '{"text":" ","class":"awake","tooltip":"staying awake: no lock or screen-off\nleft-click to turn off"}' ;;
            *)     printf '%s\n' '{"text":" ","class":"off","tooltip":"left-click: stay awake\nright-click: game mode"}' ;;
          esac
          ;;
        left)
          lock
          case $(state) in
            # starting it stops swayidle; if it fails, bring swayidle back
            off)   systemctl --user start awake.service || systemctl --user start swayidle.service ;;
            awake) systemctl --user start swayidle.service ;;   # stops awake (Conflicts)
            game)  ;;                                            # right-click turns it off
          esac
          bar
          ;;
        right)
          lock
          case $(state) in
            off)
              systemctl --user start awake-game.service
              bar
              # gamemoded -r exits straight away if GameMode can't be turned
              # on; awake.service stays up, so the sliver goes back to amber
              sleep 1
              on awake-game.service ||
                notify-send -u critical "game mode" "GameMode didn't start, only staying awake"
              ;;
            game)  systemctl --user start swayidle.service ;;   # stops awake, and with it game
            awake) ;;                                            # left-click turns it off
          esac
          bar
          ;;
      esac
    '';
  };
in
{
  home.packages = [ awake ];

  systemd.user.services.awake = {
    Unit = {
      Description = "Stay awake: swayidle that only locks before sleep";
      Conflicts = [ "swayidle.service" ];
      After = [ "sway-session.target" ];
      PartOf = [ "sway-session.target" ];
    };
    Service = {
      ExecStart = "${pkgs.swayidle}/bin/swayidle -w before-sleep '${lock}' lock '${lock}'";
      # swayidle runs its commands with sh -c
      Environment = [ "PATH=${pkgs.lib.makeBinPath [ pkgs.bash ]}" ];
    };
  };

  systemd.user.services.awake-game = {
    Unit = {
      Description = "Game mode: stay awake with GameMode on";
      Requires = [ "awake.service" ];
      After = [ "awake.service" ];
      PartOf = [ "awake.service" ];
    };
    # asks GameMode to stay on, then waits until stopped
    Service.ExecStart = "${pkgs.gamemode}/bin/gamemoded -r";
  };
}
