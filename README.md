# nixos-config

My personal NixOS configuration using flakes and Home Manager.

## Structure

```
/etc/nixos/
├── flake.nix                  # Flake inputs (nixpkgs, home-manager, plasma-manager)
├── configuration.nix          # System config (hardware, services, users)
├── home.nix                   # Home Manager root
├── home/
│   ├── shell.nix              # Zsh, aliases, PATH
│   ├── autostart.nix          # Systemd autostart services
│   ├── gtk.nix                # GTK3/GTK4 theming
│   ├── plasma.nix             # KDE Plasma settings
│   ├── packages.nix           # User packages
│   └── local-apps.nix        # Manually downloaded binaries (uwuplsplay, stremio-cliuwu, zipline-upload)
├── hosts/
│   └── pc-system.nix         # Machine-specific config (drives, etc.) — commented out by default
└── modules/
    ├── core-packages.nix      # Essential packages
    ├── gaming.nix             # Steam, Lutris, GameMode, etc.
    ├── media.nix              # mpv, ffmpeg, OBS, Spotify, etc.
    ├── social.nix             # Vesktop, Telegram
    ├── dev.nix                # Rust, Go, Python
    ├── amd.nix                # AMD GPU drivers
    ├── nvidia.nix             # NVIDIA GPU drivers (commented out)
    └── intel.nix              # Intel GPU drivers (commented out)
```

## Fresh Install

1. Install NixOS with the graphical installer
2. Clone this repo:
   ```bash
   sudo git clone https://github.com/ilovealienz/nixos-config /etc/nixos
   ```
3. Generate hardware config:
   ```bash
   sudo nixos-generate-config --show-hardware-config > /etc/nixos/hardware-configuration.nix
   ```
4. Set your GPU module in `configuration.nix` — uncomment one of:
   - `./modules/amd.nix`
   - `./modules/nvidia.nix`
   - `./modules/intel.nix`
5. Rebuild:
   ```bash
   sudo nixos-rebuild switch --flake /etc/nixos#nixos
   ```
6. Set your password:
   ```bash
   passwd
   ```

## Machine-Specific Config (for my PC)

To enable drives and machine-specific settings:

```bash
~/enable-pc-host.sh
nxrebuild
```

## Aliases

| Alias | Command |
|---|---|
| `nxrebuild` | Rebuild and switch to new config |
| `nxupdate` | Update flake inputs and rebuild |
| `nxpush` | Commit and push config to GitHub |

## MPV Config

MPV config is automatically cloned from [my-mpv-config](https://github.com/ilovealienz/my-mpv-config) on first rebuild.
