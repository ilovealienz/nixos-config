{ osConfig, ... }:
let c = osConfig.theme.colors; in
{
  services.mako = {
    enable = true;
    settings = {
      background-color = "#${c.bg}";
      text-color = "#${c.fg}";
      border-color = "#${c.accent}";
      border-size = 2;
      border-radius = 0;
      font = "Inter 11";
      padding = "10";
      margin = "8";
      default-timeout = 5000;
      width = 350;
      height = 150;
      progress-color = "over #${c.accentDim}";
      on-button-left = "exec makoctl dismiss --no-history -n $id";

      max-history = 10;

      "urgency=high" = {
        border-color = "#${c.red}";
        text-color = "#${c.red}";
      };

      "mode=dnd" = {
        invisible = true;
      };
      "mode=dnd category=osd" = {
        invisible = false;
      };
      "category=osd" = {
        history = false;
      };
    };
  };
}
