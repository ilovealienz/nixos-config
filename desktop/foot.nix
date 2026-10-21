# foot: lighter terminal for popups and Super+Shift+Return; looks like kitty.
{ osConfig, ... }:
let
  c = osConfig.theme.colors;
  dark = osConfig.theme.variant == "dark";
  colors = {
    foreground = c.fg;
    background = c.bg;
    # black: dark text on a light background
    regular0 = if dark then c.surface else c.fgBright;
    regular1 = c.red;
    regular2 = c.green;
    regular3 = c.orange;
    regular4 = c.blue;
    regular5 = c.magenta;
    regular6 = c.yellow;
    regular7 = c.muted;
    bright0 = if dark then c.surface else c.muted;
    bright1 = c.red;
    bright2 = c.green;
    bright3 = c.accent;
    bright4 = c.blue;
    bright5 = c.magenta;
    bright6 = c.yellow;
    bright7 = c.muted;
    # same as GTK's selection
    selection-foreground = c.selectionFg;
    selection-background = c.selectionBg;
    # text under the cursor, then the cursor
    cursor = "${c.bg} ${c.fg}";
  };
in
{
  programs.foot = {
    enable = true;
    settings = {
      main.font = "MonaspiceAr Nerd Font:size=12";
      # the palette already follows the theme's variant
      colors-dark = colors;
      colors-light = colors;
    };
  };
}
