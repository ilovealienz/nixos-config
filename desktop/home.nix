{ ... }:
{
  imports = [
    # sway itself, then its tools (each with its keys); the monitors are
    # per machine, imported by hosts/<host>.nix
    ./sway.nix
    ./menus.nix
    ./wallpaper.nix
    ./lock.nix
    ./nightlight.nix
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
