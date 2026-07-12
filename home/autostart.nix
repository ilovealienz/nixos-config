{ pkgs, ... }:

{
  systemd.user.services.spotify-delayed = {
    Unit = {
      Description = "Spotify delayed autostart";
      After = "graphical-session.target";
    };
    Service = {
      Type = "simple";
      ExecStartPre = "${pkgs.coreutils}/bin/sleep 5";
      ExecStart = "${pkgs.bash}/bin/bash -c 'rm -rf \${HOME}/.cache/spotify && exec ${pkgs.spotify}/bin/spotify'";
    };
    Install = {
      WantedBy = [ "graphical-session.target" ];
    };
  };

  systemd.user.services.signal-delayed = {
    Unit = {
      Description = "Signal delayed autostart";
      After = "graphical-session.target";
    };
    Service = {
      Type = "simple";
      ExecStartPre = "${pkgs.coreutils}/bin/sleep 3";
      ExecStart = "${pkgs.signal-desktop}/bin/signal-desktop";
    };
    Install = {
      WantedBy = [ "graphical-session.target" ];
    };
  };
}
