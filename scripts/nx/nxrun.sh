# nxrun <package> [args]: run a nixpkgs package without installing it
[ $# -ge 1 ] || { echo "usage: nxrun <package> [args]" >&2; exit 1; }
pkg=$1; shift
exec nix run "nixpkgs#$pkg" -- "$@"
