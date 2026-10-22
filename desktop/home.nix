{ ... }:
{
  imports = [
    # sway, then its tools; monitors come from hosts/<host>.nix
    ./sway.nix
    ./menus.nix
    ./wallpaper.nix
    ./lock.nix
    ./nightlight.nix
    ./location.nix
    ./osd.nix
    ./emoji.nix
    ./audio-menu.nix
    ./screenshot.nix
    ./screenrec.nix
    ./window-picker.nix
    ./dnd.nix
    ./placement.nix
    ./waybar.nix
    ./fuzzel.nix
    ./mako.nix
    ./gtk.nix
    ./kitty.nix
    ./foot.nix
    ./weather.nix
    ./birthdays.nix
    ./updates.nix
    ./sway-helpers.nix
    ./shortcuts.nix
    ./msqu.nix
    ./theme-edit.nix
    ./popout.nix
    ./session.nix
    ./awake.nix
    ./power.nix
    ./extract.nix
  ];
}
