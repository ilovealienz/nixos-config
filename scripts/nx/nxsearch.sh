# nxsearch [query]: search nixpkgs, print the package picked
# (no pipefail: the list is cut off when fzf exits, which isn't an error)
set +o pipefail
nix-search-tv print \
  | fzf --ansi --reverse --query "${1:-}" --preview 'nix-search-tv preview {}' \
  | sed 's|nixpkgs/||'
