{ pkgs, ... }:
{
  home.sessionPath = [ "$HOME/.bin" ];

  programs.zsh = {
    enable = true;
    enableCompletion = true;
    autosuggestion.enable = true;
    syntaxHighlighting.enable = true;

    history = {
      size = 50000;
      save = 50000;
      ignoreDups = true;
      ignoreSpace = true;
      share = true;
    };

    historySubstringSearch = {
      enable = true;
      searchUpKey = [ "^[[A" ];
      searchDownKey = [ "^[[B" ];
    };

    shellAliases = {
      nxupdate = "cd /etc/nixos && sudo nix flake update && nxrebuild";
      fpup = "flatpak update";
      v = "nvim";
    };

    initContent = ''
      # ── prompt (robbyrussell-equivalent, no framework) ──
      setopt PROMPT_SUBST

      git_prompt() {
        local ref dirty
        ref=$(git symbolic-ref --short HEAD 2>/dev/null) || return
        [[ -n $(git status --porcelain 2>/dev/null) ]] && dirty=' %F{yellow}✗%f'
        echo "%F{blue}git:(%F{red}''${ref}%F{blue})%f''${dirty}"
      }


      PROMPT='%(?:%F{green}➜:%F{red}➜)%f %F{cyan}%c%f $(git_prompt) '

      # ── functions ──
      # hardware-configuration.nix is per machine: never committed, but kept
      # staged, because a flake only sees files git knows about
      nxrebuild() {
        rm -f ~/.gtkrc-2.0.backup
        sudo git -C /etc/nixos add -f hardware-configuration.nix &&
          sudo nixos-rebuild switch --flake "/etc/nixos#$(hostname)"
      }
      # pull and rebuild. flake.lock comes from the repo: one changed here
      # (an nxupdate that wasn't pushed) is dropped first
      nxpull() {
        local g=(sudo -E git -C /etc/nixos)
        if ! "''${g[@]}" diff --quiet -- flake.lock; then
          echo "nxpull: dropping this machine's unpushed flake.lock update"
          "''${g[@]}" checkout -q -- flake.lock || return
        fi
        if ! GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' "''${g[@]}" pull --ff-only; then
          echo "nxpull: not pulled; see git -C /etc/nixos status"
          return 1
        fi
        nxrebuild
      }
      # nxpush [message]: commit everything except hardware-configuration.nix
      # and .gitignore'd files, then push (also earlier unpushed commits)
      nxpush() {
        local g=(sudo -E git -C /etc/nixos)
        local skip=(-- . ':(exclude)hardware-configuration.nix')
        "''${g[@]}" add -A || return
        if ! "''${g[@]}" diff --cached --quiet "''${skip[@]}"; then
          "''${g[@]}" commit -q -m "''${1:-update}" "''${skip[@]}" || return
        fi
        GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' "''${g[@]}" push
      }
      nxedit() { local out key f; while out=$(cd /etc/nixos && fd -e nix | awk -F/ '{ if (NF==1) printf "0\t\033[1;38;2;163;190;140m%s\033[0m\n",$0; else { d=$0; sub("/[^/]*$","",d); n=$0; sub(".*/","",n); printf "1\t\033[38;2;129;161;193m%s/\033[0m\033[38;2;216;222;233m%s\033[0m\n",d,n } }' | sort | cut -f2- | fzf --ansi --reverse --expect=ctrl-r,ctrl-a --prompt='➜ ' --header 'enter edit · ^r rebuild · ^a git add' --preview 'bat --color=always --style=numbers /etc/nixos/{}' --preview-window 'right,55%,border-left' --color='gutter:-1,hl:#88c0d0,fg+:#eceff4,bg+:#3b4252,hl+:#8fbcbb,pointer:#bf616a,marker:#a3be8c,header:#81a1c1,border:#4c566a,prompt:#81a1c1'); do key=$(head -1 <<< "$out"); f=$(sed -n 2p <<< "$out"); [[ -z "$f" ]] && continue; case "$key" in ctrl-r) rm -f ~/.gtkrc-2.0.backup; sudo git -C /etc/nixos add -f hardware-configuration.nix && sudo nixos-rebuild switch --flake /etc/nixos#$(hostname) ;; ctrl-a) sudo git -C /etc/nixos add -f "$f" && echo "staged $f" ;; *) ''${EDITOR:-nvim} "/etc/nixos/$f" ;; esac; done; }
      nxrun() { nix run nixpkgs#"$1" -- "''${@:2}"; }
      nxaudio() { [ $# -eq 0 ] && set -- -c 0; alsamixer "$@" && sudo alsactl store && echo "mixer saved"; }
      ldrun() { $NIX_LD --library-path "$NIX_LD_LIBRARY_PATH" "$@"; }
      nxclean() { if [ "$EUID" -eq 0 ]; then echo "Do not run nxclean as root/sudo"; return 1; fi; nh clean all -k 3; }
      nxsrun() { nix-search-tv print | fzf --ansi --preview 'nix-search-tv preview {}' --reverse --query "''${1:-}" | sed 's|nixpkgs/||' | xargs -I{} nix run nixpkgs#{}; }
      nxsearch() { nix-search-tv print | fzf --ansi --preview 'nix-search-tv preview {}' --reverse --query "''${1:-}" | sed 's|nixpkgs/||'; }
      nxport() { [ -z "''${1:-}" ] && { echo "usage: nxport <port> [close]"; return 1; }; if [ "''${2:-}" = "close" ]; then while sudo iptables -C nixos-fw -p tcp --dport "$1" -j ACCEPT 2>/dev/null; do sudo iptables -D nixos-fw -p tcp --dport "$1" -j ACCEPT; done; while sudo iptables -C nixos-fw -p udp --dport "$1" -j ACCEPT 2>/dev/null; do sudo iptables -D nixos-fw -p udp --dport "$1" -j ACCEPT; done; echo "closed $1"; else sudo iptables -C nixos-fw -p tcp --dport "$1" -j ACCEPT 2>/dev/null || sudo iptables -I nixos-fw -p tcp --dport "$1" -j ACCEPT; sudo iptables -C nixos-fw -p udp --dport "$1" -j ACCEPT 2>/dev/null || sudo iptables -I nixos-fw -p udp --dport "$1" -j ACCEPT; echo "opened $1 (tcp+udp) — until next rebuild/reboot"; fi; }	

      [[ -f ~/.aliases ]] && source ~/.aliases
    '';
  };
}
