# nxsrun [query]: search nixpkgs and run the package picked
pick=$(nxsearch "${1:-}") || exit 0
[ -n "$pick" ] || exit 0
exec nix run "nixpkgs#$pick"
