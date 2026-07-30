{ ... }:
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
        background = "24221cee";
        text = "d4b07bff";
        match = "e5a440ff";
        selection = "473f31ff";
        selection-text = "ede0c8ff";
        selection-match = "e5a440ff";
        border = "e5a440ff";
      };
      border = {
        width = 2;
        radius = 0;
      };
    };
  };
}
