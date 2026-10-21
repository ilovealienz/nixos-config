# nxrebuild: rebuild and switch this machine
nixos=/etc/nixos
rm -f ~/.gtkrc-2.0.backup

# files written with sudo (git pull, patches) back to you, so they can be
# edited and saved
sudo find "$nixos" -path "$nixos/.git" -prune -o ! -user "$(id -un)" \
  -exec chown "$(id -un):$(id -gn)" {} +

# your colors.json as local-theme.json: staged so the flake sees it, never
# committed (no colors.json: theme.nix's defaults)
src=~/.config/theme/colors.json
dst=$nixos/local-theme.json
if [ -f "$src" ]; then
  cmp -s "$src" "$dst" || sudo install -m 644 "$src" "$dst"
  sudo git -C "$nixos" add -f local-theme.json
elif [ -e "$dst" ]; then
  sudo git -C "$nixos" rm -q --cached --ignore-unmatch local-theme.json
  sudo rm -f "$dst"
fi

# hardware-configuration.nix: likewise staged, never committed
sudo git -C "$nixos" add -f hardware-configuration.nix
sudo nixos-rebuild switch --flake "$nixos#$(hostname)"
