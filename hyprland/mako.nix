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

      # urgent notifications get the red accent
      "urgency=high" = {
        border-color = "#e56b55";
        text-color = "#e56b55";
      };
    };
  };
}
