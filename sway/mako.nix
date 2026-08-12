{ ... }:
{
  services.mako = {
    enable = true;
    settings = {
      background-color = "#24221c";
      text-color = "#d4b07b";
      border-color = "#e5a440";
      border-size = 2;
      border-radius = 0;
      font = "Inter 11";
      padding = "10";
      margin = "8";
      default-timeout = 5000;
      width = 350;
      height = 150;
      progress-color = "over #523e20";
      on-button-left = "exec makoctl dismiss --no-history -n $id";

      max-history = 10;

      "urgency=high" = {
        border-color = "#e56b55";
        text-color = "#e56b55";
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
