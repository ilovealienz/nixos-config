# theme-edit: colours, presets, icons and wallpaper with a live preview.
# Saves ~/.config/theme/colors.json; nxrebuild applies it.
# Source: ../pkgs/theme-edit/
{ pkgs, lib, ... }:
let
  themeEdit = pkgs.rustPlatform.buildRustPackage {
    pname = "theme-edit";
    version = "1.5.0";
    src = ../pkgs/theme-edit;
    cargoLock.lockFile = ../pkgs/theme-edit/Cargo.lock;
    # loaded at runtime
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
