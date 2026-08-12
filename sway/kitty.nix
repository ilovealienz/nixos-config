{ ... }:
{
  programs.kitty = {
    enable = true;
    font = {
      name = "MonaspiceAr Nerd Font";
      size = 12;
    };
    settings = {
      # DesertNight by sainnhe
      foreground = "#d4b07b";
      background = "#24221c";

      color0  = "#473f31";  color8  = "#473f31";
      color1  = "#e56b55";  color9  = "#e56b55";
      color2  = "#99b05f";  color10 = "#99b05f";
      color3  = "#e18245";  color11 = "#e5a440";
      color4  = "#949fb4";  color12 = "#949fb4";
      color5  = "#d261a5";  color13 = "#d261a5";
      color6  = "#bfab36";  color14 = "#bfab36";
      color7  = "#87765d";  color15 = "#87765d";

      active_tab_foreground   = "#eeeeee";
      active_tab_background   = "#2b2922";
      inactive_tab_foreground = "#d4b07b";
      inactive_tab_background = "#1d1b16";

      cursor = "#d4b07b";
      selection_background = "#473f31";
      selection_foreground = "#24221c";
    };
  };
}
