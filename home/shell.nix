{ pkgs, ... }:

{
  home.sessionPath = [ "$HOME/.bin" ];

  programs.zsh = {
    enable = true;
    oh-my-zsh = {
      enable = true;
      theme = "robbyrussell";
      plugins = [
        "git"
        "z"
      ];
    };
    plugins = [
      {
        name = "zsh-autosuggestions";
        src = pkgs.zsh-autosuggestions;
        file = "share/zsh-autosuggestions/zsh-autosuggestions.zsh";
      }
      {
        name = "zsh-syntax-highlighting";
        src = pkgs.zsh-syntax-highlighting;
        file = "share/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh";
      }
    ];
    shellAliases = {
      nxrebuild = "rm -f ~/.gtkrc-2.0.backup; sudo git -C /etc/nixos add -f hardware-configuration.nix && sudo nixos-rebuild switch --flake /etc/nixos#$(hostname)";
      nxupdate = "cd /etc/nixos && sudo nix flake update && nxrebuild";
      nxpull = "GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' sudo -E git -C /etc/nixos pull && nxrebuild";
      nxpush = "GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' sudo -E git -C /etc/nixos add -f . && sudo git -C /etc/nixos rm --cached hardware-configuration.nix 2>/dev/null; sudo git -C /etc/nixos commit -m 'update' && GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' sudo -E git -C /etc/nixos push";
      fpup = "flatpak update";
      v = "nvim";
    };
    initContent = ''
      nxedit() { local out key f; while out=$(cd /etc/nixos && fd -e nix | awk -F/ '{ if (NF==1) printf "0\t\033[1;38;2;163;190;140m%s\033[0m\n",$0; else { d=$0; sub("/[^/]*$","",d); n=$0; sub(".*/","",n); printf "1\t\033[38;2;129;161;193m%s/\033[0m\033[38;2;216;222;233m%s\033[0m\n",d,n } }' | sort | cut -f2- | fzf --ansi --reverse --expect=ctrl-r,ctrl-a --prompt='➜ ' --header 'enter edit · ^r rebuild · ^a git add' --preview 'bat --color=always --style=numbers /etc/nixos/{}' --preview-window 'right,55%,border-left' --color='gutter:-1,hl:#88c0d0,fg+:#eceff4,bg+:#3b4252,hl+:#8fbcbb,pointer:#bf616a,marker:#a3be8c,header:#81a1c1,border:#4c566a,prompt:#81a1c1'); do key=$(head -1 <<< "$out"); f=$(sed -n 2p <<< "$out"); [[ -z "$f" ]] && continue; case "$key" in ctrl-r) rm -f ~/.gtkrc-2.0.backup; sudo git -C /etc/nixos add -f hardware-configuration.nix && sudo nixos-rebuild switch --flake /etc/nixos#$(hostname) ;; ctrl-a) sudo git -C /etc/nixos add -f "$f" && echo "staged $f" ;; *) ''${EDITOR:-nvim} "/etc/nixos/$f" ;; esac; done; }
      nxrun() { nix run nixpkgs#"$1" -- "''${@:2}"; }
      ldrun() { $NIX_LD --library-path "$NIX_LD_LIBRARY_PATH" "$@"; }
      nxclean() { if [ "$EUID" -eq 0 ]; then echo "Do not run nxclean as root/sudo"; return 1; fi; nh clean all -k 3; }
      nxsrun() { nix-search-tv print | fzf --ansi --preview 'nix-search-tv preview {}' --reverse --query "''${1:-}" | sed 's|nixpkgs/||' | xargs -I{} nix run nixpkgs#{}; }
      nxsearch() { nix-search-tv print | fzf --ansi --preview 'nix-search-tv preview {}' --reverse --query "''${1:-}" | sed 's|nixpkgs/||'; }
      [[ -f ~/.aliases ]] && source ~/.aliases
    '';
  };
}
