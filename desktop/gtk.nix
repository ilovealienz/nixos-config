{ pkgs, osConfig, ... }:
let
  c = osConfig.theme.colors;
  dark = osConfig.theme.variant == "dark";
  # adw-gtk3's colours, for GTK 3 and 4 alike
  palette = ''
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
    @define-color accent_fg_color #${c.onAccent};
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
    @define-color popup_bg_color #${c.bgAlt};'';
  # the icon set picked in theme-edit (theme.nix's iconSets)
  icons = osConfig.theme.icon.name;
  iconPkg = pkgs.${osConfig.theme.icon.package};
  # stops the rebuild, instead of leaving apps without icons, if the
  # package doesn't have the theme (renamed in a newer release, say)
  iconCheck = pkgs.runCommandLocal "icon-theme-check" { } ''
    if [ ! -f ${iconPkg}/share/icons/${icons}/index.theme ]; then
      echo "theme.nix iconSets: ${osConfig.theme.icon.package} has no icon theme ${icons}" >&2
      exit 1
    fi
    mkdir $out
  '';
in
{
  # ── GTK theme (dark or light, from theme.nix) + compact headerbar CSS ──
  gtk = {
    enable = true;
    theme = {
      name = if dark then "adw-gtk3-dark" else "adw-gtk3";
      package = pkgs.adw-gtk3;
    };
    iconTheme = {
      name = icons;
      package = iconPkg;
    };
    gtk3.extraCss = ''
      ${palette}
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
      gtk-recent-files-enabled = 0;
      gtk-application-prefer-dark-theme = if dark then 1 else 0;
    };
    gtk4.extraCss = ''
      ${palette}
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
      gtk-recent-files-enabled = 0;
      gtk-application-prefer-dark-theme = if dark then 1 else 0;
    };
  };

  home.packages = [ iconCheck ] ++ map (p: pkgs.${p}) osConfig.theme.icon.also;

  # ── libadwaita / GNOME apps, and Brave/Electron through the portal ──
  dconf.settings."org/gnome/desktop/interface" = {
    color-scheme = if dark then "prefer-dark" else "prefer-light";
    icon-theme = icons;
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

  # ── Qt apps ──
  qt = {
    enable = true;
    platformTheme.name = "adwaita";
    style.name = if dark then "adwaita-dark" else "adwaita";
  };
}
