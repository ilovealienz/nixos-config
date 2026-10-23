# nixos-config

My personal NixOS config (sway, home-manager).

New machine:

```
curl -LO https://raw.githubusercontent.com/ilovealienz/nixos-config/main/bootstrap.sh
chmod +x bootstrap.sh && sudo ./bootstrap.sh
```

Day to day:

```
nxrebuild          rebuild and switch
nxpull             pull from GitHub, then rebuild
nxpush "message"   check, commit and push
nxsecrets unpack   weather key and birthdays, once per machine (passphrase in Bitwarden)
nxgs               a game at the wrong size: GE-Proton, Proton Wayland or gamescope
```

Super+/ lists every shortcut and command.
