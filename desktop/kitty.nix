{ osConfig, ... }:
let
  c = osConfig.theme.colors;
  dark = osConfig.theme.variant == "dark";
in
{
  programs.kitty = {
    enable = true;
    font = {
      name = "MonaspiceAr Nerd Font";
      size = 12;
    };
    settings = {
      foreground = "#${c.fg}";
      background = "#${c.bg}";

      # black: dark text on a light background
      color0  = "#${if dark then c.surface else c.fgBright}";  color8  = "#${if dark then c.surface else c.muted}";
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
      # same as GTK's selection
      selection_background = "#${c.selectionBg}";
      selection_foreground = "#${c.selectionFg}";
    };
  };
}
