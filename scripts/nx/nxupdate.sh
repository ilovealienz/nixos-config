# nxupdate: the newest packages (flake inputs), rebuild, then the flatpaks
sudo nix flake update --flake /etc/nixos

# Spotifast isn't in nixpkgs: pkgs/spotifast.nix moves to its newest release
spotifast() {
  local f=/etc/nixos/pkgs/spotifast.nix cur new url hash
  cur=$(sed -n 's/^  version = "\(.*\)";$/\1/p' "$f")
  new=$(git ls-remote --tags --refs https://github.com/crmne/spotifast 2>/dev/null \
    | sed -n 's#.*refs/tags/v\([0-9]*\.[0-9]*\.[0-9]*\)$#\1#p' | sort -V | tail -n 1 || true)
  if [ -z "$cur" ] || [ -z "$new" ]; then
    echo "spotifast: couldn't check for a newer release"
    return 0
  fi
  if [ "$new" = "$cur" ] || [ "$(printf '%s\n%s\n' "$cur" "$new" | sort -V | tail -n 1)" != "$new" ]; then
    return 0
  fi
  url="https://github.com/crmne/spotifast/releases/download/v$new/spotifast-v$new-x86_64-unknown-linux-gnu.tar.gz"
  # the release build can come a while after the tag
  if ! hash=$(nix store prefetch-file --json "$url" 2>/dev/null | jq -er .hash); then
    echo "spotifast $new: no Linux build yet, staying on $cur"
    return 0
  fi
  sudo sed -i -e "s|^  version = \"$cur\";|  version = \"$new\";|" \
    -e "s|^    hash = \"sha256-[^\"]*\";|    hash = \"$hash\";|" "$f"
  echo "spotifast $cur -> $new"
}
spotifast
nxrebuild
echo
flatpak update -y
# the bar's update block
pkill -RTMIN+11 waybar || true
