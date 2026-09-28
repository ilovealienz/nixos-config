{ osConfig, ... }:
let c = osConfig.theme.colors; in
{
  programs.fuzzel = {
    enable = true;
    settings = {
      main = {
        font = "Inter:size=12";
        terminal = "kitty";
        layer = "overlay";
        icons-enabled = "no";
        x-margin = 8;
        y-margin = 26;              # 24px bar + small gap; now respected
        width = 35;
        horizontal-pad = 20;
        vertical-pad = 12;
        inner-pad = 8;
      };
      colors = {
        background = "${c.bg}ee";
        text = "${c.fg}ff";
        match = "${c.accent}ff";
        selection = "${c.surface}ff";
        selection-text = "${c.fgBright}ff";
        selection-match = "${c.accent}ff";
        border = "${c.accent}ff";
      };
      border = {
        width = 2;
        radius = 0;
      };
    };
  };
}
