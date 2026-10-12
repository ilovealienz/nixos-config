{ pkgs, ... }:
let
  # While a game runs, GameMode asks power-profiles-daemon to hold the
  # performance profile; when the hold ends, power-profiles-daemon goes
  # back to the profile you picked. The hold ends when the game does, if
  # GameMode stops, or if you pick a profile yourself meanwhile.
  # GameMode doesn't change the CPU governor or platform profile itself:
  # that needs the gamemode group, which you're not in, so
  # power-profiles-daemon stays in charge of the CPU.
  # Games use GameMode through Steam's launch options: gamemoderun %command%
  gamemodeHold = pkgs.writeShellScript "gamemode-power-hold" ''
    # gamemoded's service has only pkexec on its PATH
    export PATH=${pkgs.lib.makeBinPath (with pkgs; [ coreutils gnugrep gawk util-linux bash power-profiles-daemon ])}
    pidfile="''${XDG_RUNTIME_DIR:-/tmp}/gamemode-power-hold.pid"
    release() {
      pid=$(cat "$pidfile" 2>/dev/null) || return 0
      rm -f "$pidfile"
      if ${pkgs.gnugrep}/bin/grep -qs powerprofilesctl "/proc/$pid/cmdline"; then
        kill "$pid"
      fi
    }
    case "$1" in
      start)
        release   # one left over from a crash
        # GameMode runs this through sh -c: it's the parent, or the
        # grandparent if sh didn't replace itself with this script
        wait="${pkgs.coreutils}/bin/sleep infinity"
        for gm in "$PPID" "$(${pkgs.gawk}/bin/awk '/^PPid:/ { print $2 }' "/proc/$PPID/status")"; do
          if ${pkgs.gnugrep}/bin/grep -qs gamemode "/proc/$gm/comm"; then
            wait="${pkgs.coreutils}/bin/tail --pid=$gm -f /dev/null"
            break
          fi
        done
        # fully detached: GameMode waits for this script's output to close
        ${pkgs.util-linux}/bin/setsid -f ${pkgs.bash}/bin/bash -c '
          echo $$ > "$1"
          exec ${pkgs.power-profiles-daemon}/bin/powerprofilesctl launch \
            -p performance -r "game running" -i gamemode $2
        ' _ "$pidfile" "$wait" </dev/null >/dev/null 2>&1
        ;;
      end)
        release
        ;;
    esac
  '';
in
{
  programs.steam.enable = true;
  programs.gamemode = {
    enable = true;
    settings.custom = {
      start = "${gamemodeHold} start";
      end = "${gamemodeHold} end";
    };
    # nothing provides org.freedesktop.ScreenSaver on sway; fullscreen
    # windows keep the screen awake through sway's inhibit_idle instead
    settings.general.inhibit_screensaver = 0;
  };

  services.sunshine = {
    enable = true;
    autoStart = false;
    capSysAdmin = true;
    openFirewall = true;
  };

  services.udev.extraRules = ''
    # GameCube Controller Adapter
    SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0337", TAG+="uaccess"

    # Wiimotes / DolphinBar
    SUBSYSTEM=="hidraw*", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0306", TAG+="uaccess"
    SUBSYSTEM=="hidraw*", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0330", TAG+="uaccess"
  '';

  environment.systemPackages = with pkgs; [
    (prismlauncher.override { jdks = [ jdk25 ]; })
    protonup-qt
    dolphin-emu
  ];
}
