# nxupdate: the newest packages (flake inputs), rebuild, then the flatpaks
sudo nix flake update --flake /etc/nixos
nxrebuild
echo
flatpak update -y
# the bar's update block
pkill -RTMIN+11 waybar || true
