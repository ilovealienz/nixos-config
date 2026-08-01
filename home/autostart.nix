{ pkgs, lib, ... }:
let
  # toggle
  autostart = {
    spotify = true;
    signal  = true;
  };

  mkDelayed = { description, delay, exec }: {
    Unit = {
      Description = description;
      After = "graphical-session.target";
    };
    Service = {
      Type = "simple";
      ExecStartPre = "${pkgs.coreutils}/bin/sleep ${toString delay}";
      ExecStart = exec;
    };
    Install = {
      WantedBy = [ "graphical-session.target" ];
    };
  };
in
{
  systemd.user.services = lib.optionalAttrs autostart.spotify {
    spotify-delayed = mkDelayed {
      description = "Spotify delayed autostart";
      delay = 5;
      exec = "${pkgs.spotify}/bin/spotify";
    };
  } // lib.optionalAttrs autostart.signal {
    signal-delayed = mkDelayed {
      description = "Signal delayed autostart";
      delay = 3;
      exec = "${pkgs.signal-desktop}/bin/signal-desktop";
    };
  };
}
