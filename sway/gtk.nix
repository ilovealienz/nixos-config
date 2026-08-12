{ pkgs, ... }:
{
  # ── GTK theme (dark) + compact headerbar CSS ──
  gtk = {
    enable = true;
    theme = {
      name = "adw-gtk3-dark";
      package = pkgs.adw-gtk3;
    };
    iconTheme = {
      name = "Gruvbox-Plus-Dark";
      package = pkgs.gruvbox-plus-icons;
    };
    gtk3.extraCss = ''
      /* ── Desert Night palette (recolours adw-gtk3 like Stylix did) ── */
      @define-color window_bg_color #24221c;
      @define-color window_fg_color #d4b07b;
      @define-color view_bg_color #2f2c24;
      @define-color view_fg_color #d4b07b;
      @define-color headerbar_bg_color #24221c;
      @define-color headerbar_fg_color #d4b07b;
      @define-color headerbar_border_color #473f31;
      @define-color headerbar_backdrop_color #2f2c24;
      @define-color popover_bg_color #2f2c24;
      @define-color popover_fg_color #d4b07b;
      @define-color card_bg_color #2f2c24;
      @define-color card_fg_color #d4b07b;
      @define-color dialog_bg_color #24221c;
      @define-color dialog_fg_color #d4b07b;
      @define-color sidebar_bg_color #2f2c24;
      @define-color sidebar_fg_color #d4b07b;
      @define-color sidebar_border_color #473f31;
      @define-color sidebar_backdrop_color #24221c;
      @define-color accent_color #e5a440;
      @define-color accent_bg_color #e5a440;
      @define-color accent_fg_color #24221c;
      @define-color destructive_color #e56b55;
      @define-color destructive_bg_color #e56b55;
      @define-color success_color #99b05f;
      @define-color warning_color #e18245;
      @define-color error_color #e56b55;
      /* legacy gtk3 names */
      @define-color theme_bg_color #24221c;
      @define-color theme_fg_color #d4b07b;
      @define-color theme_base_color #2f2c24;
      @define-color theme_text_color #d4b07b;
      @define-color theme_selected_bg_color #e5a440;
      @define-color theme_selected_fg_color #24221c;
      @define-color insensitive_bg_color #2f2c24;
      @define-color insensitive_fg_color #87765d;
      @define-color borders #473f31;
      @define-color menu_color #2f2c24;
      @define-color popup_bg_color #2f2c24;
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
      /* ── Desert Night palette (recolours adw-gtk3 like Stylix did) ── */
      @define-color window_bg_color #24221c;
      @define-color window_fg_color #d4b07b;
      @define-color view_bg_color #2f2c24;
      @define-color view_fg_color #d4b07b;
      @define-color headerbar_bg_color #24221c;
      @define-color headerbar_fg_color #d4b07b;
      @define-color headerbar_border_color #473f31;
      @define-color headerbar_backdrop_color #2f2c24;
      @define-color popover_bg_color #2f2c24;
      @define-color popover_fg_color #d4b07b;
      @define-color card_bg_color #2f2c24;
      @define-color card_fg_color #d4b07b;
      @define-color dialog_bg_color #24221c;
      @define-color dialog_fg_color #d4b07b;
      @define-color sidebar_bg_color #2f2c24;
      @define-color sidebar_fg_color #d4b07b;
      @define-color sidebar_border_color #473f31;
      @define-color sidebar_backdrop_color #24221c;
      @define-color accent_color #e5a440;
      @define-color accent_bg_color #e5a440;
      @define-color accent_fg_color #24221c;
      @define-color destructive_color #e56b55;
      @define-color destructive_bg_color #e56b55;
      @define-color success_color #99b05f;
      @define-color warning_color #e18245;
      @define-color error_color #e56b55;
      /* legacy gtk3 names */
      @define-color theme_bg_color #24221c;
      @define-color theme_fg_color #d4b07b;
      @define-color theme_base_color #2f2c24;
      @define-color theme_text_color #d4b07b;
      @define-color theme_selected_bg_color #e5a440;
      @define-color theme_selected_fg_color #24221c;
      @define-color insensitive_bg_color #2f2c24;
      @define-color insensitive_fg_color #87765d;
      @define-color borders #473f31;
      @define-color menu_color #2f2c24;
      @define-color popup_bg_color #2f2c24;
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

  # ── libadwaita / GNOME apps: prefer dark ──
  dconf.settings."org/gnome/desktop/interface" = {
    color-scheme = "prefer-dark";
    icon-theme = "Gruvbox-Plus-Dark";
    cursor-theme = "Bibata-Modern-Classic";
  };

  # ── cursor (system-wide pointer, incl. XWayland) ──
  home.pointerCursor = {
    name = "Bibata-Modern-Classic";
    package = pkgs.bibata-cursors;
    size = 24;
    gtk.enable = true;
    x11.enable = true;
  };

  # ── Qt apps: dark ──
  qt = {
    enable = true;
    platformTheme.name = "adwaita";
    style.name = "adwaita-dark";
  };
}
