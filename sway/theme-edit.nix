# Theme editor: colour pickers for every colour in theme.nix, dark and
# light, a live mock of the desktop (bar, kitty, a GTK window, fuzzel,
# mako) and contrast checks. Saves to ~/.config/theme/colors.json, which
# nxrebuild applies; "Save and rebuild" runs nxrebuild in foot.
# Run theme-edit, or "Theme editor" in the launcher.
#
# Source: ../pkgs/theme-edit/
{ pkgs, lib, ... }:
let
  themeEdit = pkgs.rustPlatform.buildRustPackage {
    pname = "theme-edit";
    version = "1.1.0";
    src = ../pkgs/theme-edit;
    cargoLock.lockFile = ../pkgs/theme-edit/Cargo.lock;
    # loaded when the window opens
    postFixup = ''
      patchelf --add-rpath ${lib.makeLibraryPath (with pkgs; [ wayland libxkbcommon libGL ])} $out/bin/theme-edit
    '';
    meta.mainProgram = "theme-edit";
  };
in
{
  home.packages = [ themeEdit ];
  xdg.desktopEntries.theme-edit = {
    name = "Theme editor";
    comment = "Pick the desktop's colours, dark or light";
    exec = "theme-edit";
    icon = "preferences-desktop-color";
    categories = [ "Settings" ];
  };
}
