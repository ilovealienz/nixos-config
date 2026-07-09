{ ... }:

{
  xdg.configFile."fastfetch/config.jsonc".text = ''
    {
      "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
      "logo": {
        "type": "command-raw",
        "source": "pokeget random --hide-name"
      },
      "display": {
        "separator": ": ",
        "color": {
          "keys": "blue"
        }
      },
      "modules": [
        "title",
        "separator",
        "os",
        "kernel",
        "uptime",
        "packages",
        "de",
        "wm",
        "terminal",
        "cpu",
        "gpu",
        "memory",
        {
          "type": "disk",
          "folders": "/"
        },
        "localip",
        "break",
        "colors"
      ]
    }
  '';
}
