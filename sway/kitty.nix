{ osConfig, ... }:
let c = osConfig.theme.colors; in
{
  programs.kitty = {
    enable = true;
    font = {
      name = "MonaspiceAr Nerd Font";
      size = 12;
    };
    settings = {
      # DesertNight by sainnhe
      foreground = "#${c.fg}";
      background = "#${c.bg}";

      color0  = "#${c.surface}";  color8  = "#${c.surface}";
      color1  = "#${c.red}";  color9  = "#${c.red}";
      color2  = "#${c.green}";  color10 = "#${c.green}";
      color3  = "#${c.orange}";  color11 = "#${c.accent}";
      color4  = "#${c.blue}";  color12 = "#${c.blue}";
      color5  = "#${c.magenta}";  color13 = "#${c.magenta}";
      color6  = "#${c.yellow}";  color14 = "#${c.yellow}";
      color7  = "#${c.muted}";  color15 = "#${c.muted}";

      active_tab_foreground   = "#${c.fgBright}";
      active_tab_background   = "#${c.bgAlt}";
      inactive_tab_foreground = "#${c.fg}";
      inactive_tab_background = "#${c.bgDark}";

      cursor = "#${c.fg}";
      selection_background = "#${c.surface}";
      selection_foreground = "#${c.bg}";
    };
  };
}
