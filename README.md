# nixos-config

my personal nixos config using flakes, home manager and plasma manager. modular setup so it's easy to add/remove stuff.

## structure

```
/etc/nixos/
├── flake.nix                  # inputs + host list (pc, laptop, generic)
├── configuration.nix          # shared system config (incl. plasma import)
├── home.nix                   # home manager root
├── home/
│   ├── shell.nix               # zsh, aliases, functions
│   ├── autostart.nix           # spotify + vesktop autostart
│   ├── gtk.nix                 # gtk3/gtk4 theming
│   ├── plasma.nix              # kde plasma settings
│   ├── packages.nix            # user packages
│   └── local-apps.nix          # uwuplsplay, stremio-cliuwu, zipline-upload
├── hosts/
│   ├── pc.nix                  # pc: amd gpu, drive mounts
│   └── laptop.nix              # laptop: intel gpu
├── desktops/
│   └── plasma.nix              # kde plasma, imported directly in configuration.nix
├── hardware/
│   ├── amd.nix                 # amd gpu (pc)
│   ├── nvidia.nix               # nvidia gpu (currently empty, fill in if needed)
│   └── intel.nix               # intel gpu (laptop)
└── programs/
    ├── core-packages.nix       # essentials
    ├── gaming.nix              # steam, lutris, bottles, gamemode, etc
    ├── media.nix               # mpv, obs, spotify, yt-dlp, etc
    ├── social.nix              # vesktop, telegram
    └── dev.nix                 # rust, go, python
```

## machines

the flake has multiple configs — each machine uses its own entry based on hostname:

- `pc` — desktop, AMD GPU, drive mounts
- `laptop` — laptop, Intel GPU
- `generic` — base config, no machine-specific hardware

## fresh install

1. install nixos with the graphical installer
2. clone this repo:
   ```bash
   sudo git clone https://github.com/ilovealienz/nixos-config /etc/nixos
   ```
3. generate your hardware config:
   ```bash
   sudo nixos-generate-config --show-hardware-config > /etc/nixos/hardware-configuration.nix
   ```
4. first rebuild — pick the right config for your machine:
   ```bash
   # for a generic setup
   sudo nixos-rebuild switch --flake /etc/nixos#generic

   # or pick your machine
   sudo nixos-rebuild switch --flake /etc/nixos#pc
   sudo nixos-rebuild switch --flake /etc/nixos#laptop
   ```
5. set your password:
   ```bash
   passwd
   ```

after the first rebuild your hostname is set automatically, so from then on just use `nxrebuild` and it picks the right config.

## gpu

each host's file in `hosts/` imports the right GPU module from `hardware/`:
- `hosts/pc.nix` → `hardware/amd.nix`
- `hosts/laptop.nix` → `hardware/intel.nix`
- nvidia → `hardware/nvidia.nix` (currently a stub, fill in `hardware.nvidia.*` options before using)

## desktop environment

`desktops/plasma.nix` is imported directly in `configuration.nix`, since it's not something that changes often. To try a different DE, copy `desktops/plasma.nix` to e.g. `desktops/hyprland.nix`, edit it for the new DE, then swap which one's imported in `configuration.nix` — and comment/uncomment `home/gtk.nix` / `home/plasma.nix` in `home.nix` to match, since those are Plasma-specific.

## aliases

| alias | what it does |
|---|---|
| `nxrebuild` | rebuild and switch (auto-detects machine from hostname) |
| `nxupdate` | update packages and rebuild |
| `nxpush` | commit and push config to github |
| `nxclean` | garbage collect old nix store paths |
| `nxrun vlc` | run a package directly by name |
| `nxsrun vlc` | fuzzy search nixpkgs and run selected package |
| `nxsearch vlc` | fuzzy search nixpkgs and return package name |
| `fpup` | update flatpak apps |

## mpv config

auto-cloned from [my-mpv-config](https://github.com/ilovealienz/my-mpv-config) on first rebuild into `~/.config/mpv`.

## local apps

these get downloaded and patched automatically on first rebuild:
- [uwuplsplay](https://github.com/ilovealienz/uwuplsplay) — stream url protocol handler
- [stremio-cliuwu](https://github.com/ilovealienz/stremio-cliuwu) — stremio cli client
- [zipline-upload](https://github.com/ilovealienz/my-zipline-uploader) — zipline file uploader
