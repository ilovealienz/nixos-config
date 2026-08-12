# nixos-config

personal nixos config — flakes + home-manager, single desktop (sway). modular so it's easy to add/remove stuff.

## structure

```
/etc/nixos/
├── flake.nix                  # inputs (nixpkgs, home-manager) + hosts (pc, laptop)
├── configuration.nix          # shared system config
├── home.nix                   # home-manager root
├── root-theme.nix             # red-tinted gtk theme for root apps (gparted etc)
├── sway/                      # the whole desktop, self-contained
│   ├── system.nix              # sway enable, ly login, portals, fonts, system pkgs
│   ├── home.nix                # imports the home-level sway files below
│   ├── compositor.nix          # keybinds, rules, swayidle/swaylock, osd + screenshot + dnd scripts
│   ├── monitors-pc.nix         # pc monitor layout + workspace pinning (host-gated)
│   ├── monitors-laptop.nix     # laptop panel (host-gated)
│   ├── waybar.nix              # status bar
│   ├── weather.nix             # waybar weather module (openweathermap)
│   ├── fuzzel.nix              # launcher
│   ├── mako.nix                # notifications
│   ├── kitty.nix               # terminal
│   └── gtk.nix                 # gtk/qt theming, icons, cursor
├── home/                      # desktop-agnostic home config
│   ├── shell.nix               # zsh, prompt, aliases, functions
│   ├── packages.nix            # user packages + nh cleanup
│   ├── autostart.nix           # spotify + signal autostart (per-app toggles)
│   ├── fastfetch.nix           # fastfetch config
│   ├── local-apps.nix          # uwuplsplay, stremio-cliuwu, zipline-upload
│   └── mime.nix                # default apps by filetype
├── hosts/
│   ├── pc.nix                  # pc: gpu, drive mounts, virt, gaming
│   └── laptop.nix              # laptop: gpu
├── hardware/
│   └── graphics.nix            # mesa (amd + intel, identical)
├── programs/                  # system packages by category
│   ├── core-packages.nix       # essentials
│   ├── gaming.nix              # steam, bottles, prism, gamemode
│   ├── virt.nix                # libvirt + virt-manager (pc only)
│   ├── media.nix               # mpv, obs, spotify, etc
│   ├── dev.nix                 # rust, go, python
│   ├── language.nix            # anki, mecab, fcitx5 japanese input
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
