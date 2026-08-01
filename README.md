# nixos-config

personal nixos config — flakes + home-manager, single desktop (hyprland). modular so it's easy to add/remove stuff.

## structure

```
/etc/nixos/
├── flake.nix                  # inputs (nixpkgs, home-manager) + hosts (pc, laptop)
├── configuration.nix          # shared system config
├── home.nix                   # home-manager root
├── hyprland/                  # the whole desktop, self-contained
│   ├── system.nix              # compositor enable, ly login, portals, fonts, system pkgs
│   ├── home.nix                # imports the home-level hyprland files below
│   ├── compositor.nix          # hyprland keybinds, rules, hyprlock/hypridle, screenshot menu
│   ├── monitors-pc.nix         # pc monitor layout + wallpaper (host-gated)
│   ├── waybar.nix              # status bar
│   ├── fuzzel.nix              # launcher
│   ├── mako.nix                # notifications
│   └── gtk.nix                 # gtk/qt theming, icons, cursor
├── home/                      # desktop-agnostic home config
│   ├── shell.nix               # zsh, aliases, functions
│   ├── packages.nix            # user packages + nh cleanup
│   ├── autostart.nix           # spotify + signal autostart
│   ├── fastfetch.nix           # fastfetch config
│   ├── local-apps.nix          # uwuplsplay, stremio-cliuwu, zipline-upload
│   └── mime.nix                # default apps by filetype
├── hosts/
│   ├── pc.nix                  # pc: amd gpu, drive mounts
│   └── laptop.nix              # laptop: intel gpu
├── hardware/
│   └── graphics.nix            # mesa (amd + intel, identical)
├── programs/                  # system packages by category
│   ├── core-packages.nix       # essentials
│   ├── gaming.nix              # steam, bottles, prism, gamemode
│   ├── media.nix               # mpv, obs, spotify, etc
│   ├── dev.nix                 # rust, go, python
│   └── social.nix              # signal, vesktop
└── walls/                     # wallpapers
```

## fresh install

```
chmod +x bootstrap.sh && sudo ./bootstrap.sh
```

pick the host (pc/laptop), it clones, generates hardware config, and builds.

## day-to-day

- `nxrebuild` — rebuild + switch
- `nxupdate` — update flake inputs + rebuild
- `nxpush` / `nxpull` — sync with github
- `nxedit` — fzf picker for config files
- `nxclean` — garbage-collect old generations (keeps 3)


## weather widget

Needs an OpenWeatherMap API key (kept out of the repo since this is public).

Get one from https://openweathermap.org — **My API keys**.

```
mkdir -p ~/.config/weather
echo "YOUR_API_KEY" > ~/.config/weather/key
chmod 600 ~/.config/weather/key
```

Can take up to 2 hours before a newly created key becomes active.
