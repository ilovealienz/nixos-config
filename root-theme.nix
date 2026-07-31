{ pkgs, ... }:
{
  # root GUI apps (gparted etc) get a red-tinted Desert Night theme
  environment.systemPackages = [ pkgs.adw-gtk3 ];

  environment.etc."root-gtk/settings.ini".text = ''
    [Settings]
    gtk-theme-name=adw-gtk3-dark
    gtk-icon-theme-name=Gruvbox-Plus-Dark
    gtk-cursor-theme-name=Bibata-Modern-Classic
    gtk-application-prefer-dark-theme=1
  '';

  environment.etc."root-gtk/gtk.css".text = ''
    @define-color window_bg_color #2a1a18;
    @define-color window_fg_color #e8c9a0;
    @define-color view_bg_color #331f1c;
    @define-color view_fg_color #e8c9a0;
    @define-color headerbar_bg_color #3a1f1c;
    @define-color headerbar_fg_color #e8c9a0;
    @define-color popover_bg_color #331f1c;
    @define-color popover_fg_color #e8c9a0;
    @define-color card_bg_color #331f1c;
    @define-color dialog_bg_color #2a1a18;
    @define-color dialog_fg_color #e8c9a0;
    @define-color sidebar_bg_color #331f1c;
    @define-color sidebar_fg_color #e8c9a0;
    @define-color accent_color #e56b55;
    @define-color accent_bg_color #e56b55;
    @define-color accent_fg_color #2a1a18;
    @define-color theme_bg_color #2a1a18;
    @define-color theme_fg_color #e8c9a0;
    @define-color theme_base_color #331f1c;
    @define-color theme_text_color #e8c9a0;
    @define-color theme_selected_bg_color #e56b55;
    @define-color theme_selected_fg_color #2a1a18;
    @define-color borders #5a2f2a;
  '';

  systemd.tmpfiles.rules = [
    "d /root/.config 0700 root root -"
    "d /root/.config/gtk-3.0 0700 root root -"
    "d /root/.config/gtk-4.0 0700 root root -"
    "L+ /root/.config/gtk-3.0/settings.ini - - - - /etc/root-gtk/settings.ini"
    "L+ /root/.config/gtk-3.0/gtk.css      - - - - /etc/root-gtk/gtk.css"
    "L+ /root/.config/gtk-4.0/settings.ini - - - - /etc/root-gtk/settings.ini"
    "L+ /root/.config/gtk-4.0/gtk.css      - - - - /etc/root-gtk/gtk.css"
  ];
}
