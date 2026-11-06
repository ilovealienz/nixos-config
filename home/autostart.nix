{ pkgs, lib, ... }:
let
  autostart = {
    spotifast = true;
    signal  = true;
  };

  mkDelayed = { description, delay, exec }: {
    Unit = {
      Description = description;
      After = "sway-session.target";
    };
    Service = {
      Type = "simple";
      ExecStartPre = "${pkgs.coreutils}/bin/sleep ${toString delay}";
      ExecStart = exec;
    };
    Install = {
      WantedBy = [ "sway-session.target" ];
    };
  };
in
{
  systemd.user.services = lib.optionalAttrs autostart.spotifast {
    spotifast-delayed = mkDelayed {
      description = "Spotifast delayed autostart";
      delay = 5;
      exec = lib.getExe (pkgs.callPackage ../pkgs/spotifast.nix { });
    };
  } // lib.optionalAttrs autostart.signal {
    signal-delayed = mkDelayed {
      description = "Signal delayed autostart";
      delay = 9;
      exec = "${pkgs.signal-desktop}/bin/signal-desktop";
    };
  };
}
