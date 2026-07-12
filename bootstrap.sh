#!/usr/bin/env bash
# nixos-config bootstrap — fresh install setup
# usage: chmod +x bootstrap.sh && sudo ./bootstrap.sh
set -euo pipefail

REPO="https://github.com/ilovealienz/nixos-config"
NIXOS_DIR="/etc/nixos"
BACKUP_DIR="/etc/nixos-backup-$(date +%s)"
TMP_CLONE="/tmp/nixos-config-clone"

# flakes are disabled on a fresh install — this enables them for every
# nix command this script runs, without needing the config applied first
export NIX_CONFIG="experimental-features = nix-command flakes"

if [ "$EUID" -ne 0 ]; then
  echo "run with sudo: sudo ./bootstrap.sh"
  exit 1
fi

echo "which machine is this?"
select HOST in pc laptop generic; do
  case "$HOST" in
    pc|laptop|generic) break ;;
    *) echo "pick 1-3" ;;
  esac
done
echo "-> building as #$HOST"

# git isn't installed on a fresh system — run the git steps inside nix-shell
nix-shell -p git --run "bash -s" << EOF
set -euo pipefail
shopt -s dotglob

# clone to temp first (can't clone into non-empty /etc/nixos)
rm -rf '$TMP_CLONE'
git clone '$REPO' '$TMP_CLONE'

# back up the installer-generated configs
mkdir -p '$BACKUP_DIR'
mv '$NIXOS_DIR'/* '$BACKUP_DIR'/

# move the clone into place (dotglob makes * include .git etc)
mv '$TMP_CLONE'/* '$NIXOS_DIR'/
rmdir '$TMP_CLONE'

# regenerate hardware config for THIS machine
nixos-generate-config --show-hardware-config > '$NIXOS_DIR/hardware-configuration.nix'

# flake needs it visible to git (it's gitignored, so force)
git -C '$NIXOS_DIR' add -f hardware-configuration.nix

# avoid 'dubious ownership' complaints later when user + root both touch the repo
git config --global --add safe.directory '$NIXOS_DIR'
EOF

echo "-> rebuilding as #$HOST (first build takes a while)..."
nixos-rebuild switch --flake "$NIXOS_DIR#$HOST"

echo ""
echo "done. old configs backed up at: $BACKUP_DIR"
echo "reboot (or log out/in) for everything to fully apply."
echo "from now on just use: nxrebuild"
echo ""
echo "note: nxpush won't work until you set up your ssh key"
echo "      (~/.ssh/id_ed25519 added to github)"
