{ pkgs, ... }:

{
  systemd.user.services.spotify-delayed = {
    Unit = {
      Description = "Spotify delayed autostart";
      After = "graphical-session.target";
    };
    Service = {
      Type = "oneshot";
      ExecStartPre = "${pkgs.coreutils}/bin/sleep 5";
      ExecStart = "${pkgs.spotify}/bin/spotify";
    };
    Install = {
      WantedBy = [ "graphical-session.target" ];
    };
  };

  systemd.user.services.vesktop-delayed = {
    Unit = {
      Description = "Vesktop delayed autostart";
      After = "graphical-session.target";
    };
    Service = {
      Type = "oneshot";
      ExecStartPre = "${pkgs.coreutils}/bin/sleep 3";
      ExecStart = "${pkgs.vesktop}/bin/vesktop";
    };
    Install = {
      WantedBy = [ "graphical-session.target" ];
    };
  };
}
