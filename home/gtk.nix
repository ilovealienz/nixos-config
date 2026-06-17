{ pkgs, ... }:

{
  gtk = {
    enable = true;
    theme = {
      name = "Breeze-Dark";
      package = pkgs.kdePackages.breeze-gtk;
    };
    gtk3.extraCss = ''
      headerbar {
          min-height: 10px;
          padding: 0;
      }
      headerbar entry,
      headerbar spinbutton,
      headerbar button,
      headerbar separator {
          margin: 0;
          padding: 0;
          min-width: 0;
          min-height: 0;
      }
      headerbar .title {
          font-family: monospace;
          font-size: 12px;
          padding: 0;
          margin: 0;
      }
      headerbar box {
          margin: 1px 0;
          padding: 0;
      }
      headerbar .titlebutton {
          min-height: 8px;
          padding: 0 2px;
      }
      headerbar button image {
          min-height: 8px;
      }
      .default-decoration {
          min-height: 0;
          padding: 0;
          margin-bottom: 0;
      }
    '';
    gtk3.extraConfig = {
      gtk-recent-files-max-age = 0;
      gtk-recent-files-limit = 0;
      gtk-application-prefer-dark-theme = 1;
    };
    gtk4.extraCss = ''
      headerbar {
          min-height: 10px;
          padding: 0px;
      }
      headerbar entry,
      headerbar spinbutton,
      headerbar button,
      headerbar separator {
          margin-top: 0px;
          margin-bottom: 0px;
          padding: 0px;
          min-width: 0px;
          min-height: 0px;
      }
      headerbar windowhandle {
          margin-top: 0px;
          margin-bottom: 0px;
          min-height: 8px;
          padding: 0px;
      }
      headerbar windowhandle box {
          margin-top: 1px;
          margin-bottom: 1px;
          padding: 0px;
      }
      headerbar windowhandle label {
          font-family: monospace;
          font-size: 12px;
          padding: 0px;
      }
      headerbar windowhandle box.end {
          margin-top: 0px;
          margin-bottom: 0px;
          margin-right: 4px;
          padding: 0px;
      }
      headerbar windowhandle box.end button image {
          min-height: 8px;
      }
      .default-decoration {
          min-height: 0;
          padding: 0px;
          margin-bottom: 0px;
      }
    '';
    gtk4.extraConfig = {
      gtk-recent-files-max-age = 0;
      gtk-recent-files-limit = 0;
      gtk-application-prefer-dark-theme = 1;
    };
  };
}
