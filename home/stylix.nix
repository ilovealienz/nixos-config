{ pkgs, ... }:
{
  stylix = {
    enable = true;
    polarity = "dark";

    # wallpaper — same image swaybg uses, so they don't disagree
    image = ../walls/mc-4k.png;

    # Desert Night palette (from your kitty theme)
    base16Scheme = {
      base00 = "24221c";  # bg
      base01 = "2f2c24";  # lighter bg
      base02 = "473f31";  # selection
      base03 = "87765d";  # dim / comments
      base04 = "9c8d70";  # dark fg
      base05 = "d4b07b";  # fg
      base06 = "e0c290";  # light fg
      base07 = "ede0c8";  # lightest
      base08 = "e56b55";  # red
      base09 = "e18245";  # orange
      base0A = "e5a440";  # amber
      base0B = "99b05f";  # green
      base0C = "bfab36";  # gold
      base0D = "949fb4";  # blue
      base0E = "d261a5";  # magenta
      base0F = "87765d";  # brown
    };

    # keep your fonts — stop Stylix swapping them for its defaults
    fonts = {
      monospace = { package = pkgs.nerd-fonts.monaspace; name = "MonaspiceAr Nerd Font"; };
      sansSerif = { package = pkgs.inter; name = "Inter"; };
      serif = { package = pkgs.inter; name = "Inter"; };
      emoji = { package = pkgs.noto-fonts-color-emoji; name = "Noto Color Emoji"; };
    };

    # muted icon theme so icons stop clashing with the brown
    iconTheme = {
      enable = true;
      package = pkgs.gruvbox-plus-icons;
      dark = "Gruvbox-Plus-Dark";
      light = "Gruvbox-Plus-Dark";
    };

    # protect the configs you hand-tuned — Stylix does GTK/Qt/apps only
    targets = {
      waybar.enable = false;
      hyprland.enable = false;
      hyprlock.enable = false;
      kitty.enable = false;
      fuzzel.enable = false;
      mako.enable = false;
    };
  };
}
