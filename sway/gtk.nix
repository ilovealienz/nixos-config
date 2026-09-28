{ pkgs, osConfig, ... }:
let c = osConfig.theme.colors; in
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
      @define-color window_bg_color #${c.bg};
      @define-color window_fg_color #${c.fg};
      @define-color view_bg_color #${c.bgAlt};
      @define-color view_fg_color #${c.fg};
      @define-color headerbar_bg_color #${c.bg};
      @define-color headerbar_fg_color #${c.fg};
      @define-color headerbar_border_color #${c.surface};
      @define-color headerbar_backdrop_color #${c.bgAlt};
      @define-color popover_bg_color #${c.bgAlt};
      @define-color popover_fg_color #${c.fg};
      @define-color card_bg_color #${c.bgAlt};
      @define-color card_fg_color #${c.fg};
      @define-color dialog_bg_color #${c.bg};
      @define-color dialog_fg_color #${c.fg};
      @define-color sidebar_bg_color #${c.bgAlt};
      @define-color sidebar_fg_color #${c.fg};
      @define-color sidebar_border_color #${c.surface};
      @define-color sidebar_backdrop_color #${c.bg};
      @define-color accent_color #${c.accent};
      @define-color accent_bg_color #${c.accent};
      @define-color accent_fg_color #${c.bg};
      @define-color destructive_color #${c.red};
      @define-color destructive_bg_color #${c.red};
      @define-color success_color #${c.green};
      @define-color warning_color #${c.orange};
      @define-color error_color #${c.red};
      /* legacy gtk3 names */
      @define-color theme_bg_color #${c.bg};
      @define-color theme_fg_color #${c.fg};
      @define-color theme_base_color #${c.bgAlt};
      @define-color theme_text_color #${c.fg};
      @define-color theme_selected_bg_color #${c.selectionBg};
      @define-color theme_selected_fg_color #${c.selectionFg};
      @define-color insensitive_bg_color #${c.bgAlt};
      @define-color insensitive_fg_color #${c.muted};
      @define-color borders #${c.surface};
      @define-color menu_color #${c.bgAlt};
      @define-color popup_bg_color #${c.bgAlt};
      /* ── selected files/rows: readable highlight (Thunar etc.) ── */
      .view:selected, .view:selected:focus, .view row:selected,
      treeview.view:selected, iconview:selected,
      .view:selected:backdrop, .view row:selected:backdrop {
          color: #${c.selectionFg};
          background-color: #${c.selectionBg};
      }
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
      @define-color window_bg_color #${c.bg};
      @define-color window_fg_color #${c.fg};
      @define-color view_bg_color #${c.bgAlt};
      @define-color view_fg_color #${c.fg};
      @define-color headerbar_bg_color #${c.bg};
      @define-color headerbar_fg_color #${c.fg};
      @define-color headerbar_border_color #${c.surface};
      @define-color headerbar_backdrop_color #${c.bgAlt};
      @define-color popover_bg_color #${c.bgAlt};
      @define-color popover_fg_color #${c.fg};
      @define-color card_bg_color #${c.bgAlt};
      @define-color card_fg_color #${c.fg};
      @define-color dialog_bg_color #${c.bg};
      @define-color dialog_fg_color #${c.fg};
      @define-color sidebar_bg_color #${c.bgAlt};
      @define-color sidebar_fg_color #${c.fg};
      @define-color sidebar_border_color #${c.surface};
      @define-color sidebar_backdrop_color #${c.bg};
      @define-color accent_color #${c.accent};
      @define-color accent_bg_color #${c.accent};
      @define-color accent_fg_color #${c.bg};
      @define-color destructive_color #${c.red};
      @define-color destructive_bg_color #${c.red};
      @define-color success_color #${c.green};
      @define-color warning_color #${c.orange};
      @define-color error_color #${c.red};
      /* legacy gtk3 names */
      @define-color theme_bg_color #${c.bg};
      @define-color theme_fg_color #${c.fg};
      @define-color theme_base_color #${c.bgAlt};
      @define-color theme_text_color #${c.fg};
      @define-color theme_selected_bg_color #${c.accent};
      @define-color theme_selected_fg_color #${c.bg};
      @define-color insensitive_bg_color #${c.bgAlt};
      @define-color insensitive_fg_color #${c.muted};
      @define-color borders #${c.surface};
      @define-color menu_color #${c.bgAlt};
      @define-color popup_bg_color #${c.bgAlt};
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
