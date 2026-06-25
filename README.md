# nixos-config

my personal nixos config using flakes, home manager and plasma manager. modular setup so it's easy to add/remove stuff.

## structure

```
/etc/nixos/
├── flake.nix                  # inputs + machine configs (pc, generic)
├── configuration.nix          # shared system config
├── home.nix                   # home manager root
├── home/
│   ├── shell.nix              # zsh, aliases, functions
│   ├── autostart.nix          # spotify + vesktop autostart
│   ├── gtk.nix                # gtk3/gtk4 theming
│   ├── plasma.nix             # kde plasma settings
│   ├── packages.nix           # user packages
│   └── local-apps.nix        # uwuplsplay, stremio-cliuwu, zipline-upload
├── hosts/
│   └── pc-system.nix         # drive mounts for my pc (pc config only)
└── modules/
    ├── core-packages.nix      # essentials
    ├── gaming.nix             # steam, lutris, bottles, gamemode, etc
    ├── media.nix              # mpv, obs, spotify, yt-dlp, etc
    ├── social.nix             # vesktop, telegram
    ├── dev.nix                # rust, go, python
    ├── amd.nix                # amd gpu (pc config only)
    ├── nvidia.nix             # nvidia gpu (commented out, swap when needed)
    └── intel.nix              # intel gpu (commented out, swap when needed)
```

## machines

the flake has multiple configs — each machine uses its own entry based on hostname:

- `pc` — my desktop (amd gpu, 3 monitors, drive mounts)
- `generic` — base config, no machine-specific stuff

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

   # or if you're setting up as my pc
   sudo nixos-rebuild switch --flake /etc/nixos#pc
   ```
5. set your password:
   ```bash
   passwd
   ```

after the first rebuild your hostname is set automatically, so from then on just use `nxrebuild` and it picks the right config.

## gpu

add the right module to your machine's entry in `flake.nix`:
- amd → `./modules/amd.nix` (already in pc config)
- nvidia → `./modules/nvidia.nix`
- intel → `./modules/intel.nix`

## aliases

| alias | what it does |
|---|---|
| `nxrebuild` | rebuild and switch (auto-detects machine from hostname) |
| `nxupdate` | update flake inputs and rebuild |
| `nxpush` | commit and push config to github |
| `nxclean` | garbage collect old nix store paths |
| `nxrun vlc` | run a package temporarily without installing |
| `nxsearch vlc` | fuzzy search nixpkgs and run selected package |
| `fpup` | update flatpak apps |

## mpv config

auto-cloned from [my-mpv-config](https://github.com/ilovealienz/my-mpv-config) on first rebuild into `~/.config/mpv`.

## local apps

these get downloaded and patched automatically on first rebuild:
- [uwuplsplay](https://github.com/ilovealienz/uwuplsplay) — stream url protocol handler
- [stremio-cliuwu](https://github.com/ilovealienz/stremio-cliuwu) — stremio cli client
- [zipline-upload](https://github.com/ilovealienz/my-zipline-uploader) — zipline file uploader
