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
      nxupdate = "sudo nix flake update --flake /etc/nixos && nxrebuild";
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
        nxtheme || return
        sudo git -C /etc/nixos add -f hardware-configuration.nix &&
          sudo nixos-rebuild switch --flake "/etc/nixos#$(hostname)"
      }
      # your colours (theme-edit's ~/.config/theme/colors.json) go to
      # /etc/nixos/local-theme.json, staged but never committed, like
      # hardware-configuration.nix; no file there: theme.nix's defaults
      nxtheme() {
        local src=~/.config/theme/colors.json dst=/etc/nixos/local-theme.json
        if [ -f "$src" ]; then
          cmp -s "$src" "$dst" || sudo install -m 644 "$src" "$dst" || return
          sudo git -C /etc/nixos add -f local-theme.json
        elif [ -e "$dst" ]; then
          sudo git -C /etc/nixos rm -q --cached --ignore-unmatch local-theme.json
          sudo rm -f "$dst"
        fi
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
        local rc=$?
        # a new weather key / birthdays from another machine (after the
        # rebuild's output, so it's seen)
        nxsecrets unpack --if-changed
        return $rc
      }
      # evaluate every host the way a rebuild would, without building
      nxcheck() {
        local flake=/etc/nixos hosts h fail=0 untracked tmp
        untracked=$(git -C "$flake" ls-files --others --exclude-standard)
        if [ -n "$untracked" ]; then
          echo "nxcheck: not seen until staged (git add):"
          print -r -- "$untracked" | sed 's/^/  /'
        fi
        hosts=($(nix eval --raw "$flake#nixosConfigurations" \
          --apply 'c: builtins.concatStringsSep " " (builtins.attrNames c)' 2>/dev/null))
        if [ ''${#hosts} -eq 0 ]; then
          echo "nxcheck: the flake itself doesn't evaluate:"
          nix eval "$flake#nixosConfigurations" --apply builtins.attrNames 2>&1 | tail -20
          return 1
        fi
        # all hosts at once: about as long as the slowest one. In a subshell
        # so the background jobs don't show up in the terminal's job list
        tmp=$(mktemp -d)
        (
          for h in $hosts; do
            { nix eval --raw "$flake#nixosConfigurations.$h.config.system.build.toplevel.drvPath" \
                >/dev/null 2>"$tmp/$h.err"; echo $? >"$tmp/$h.rc"; } &
          done
          wait
        )
        for h in $hosts; do
          if [ "$(cat "$tmp/$h.rc")" = 0 ]; then
            echo "✓ $h"
          else
            echo "✗ $h"
            grep -v "^warning: Git tree" "$tmp/$h.err" | tail -20 | sed 's/^/  /'
            fail=1
          fi
        done
        rm -rf "$tmp"
        return $fail
      }
      # nxpush [--no-check] [message]: commit everything except
      # hardware-configuration.nix and .gitignore'd files, then push (also
      # earlier unpushed commits). Every host is checked first.
      nxpush() {
        local g=(sudo -E git -C /etc/nixos)
        local skip=(-- . ':(exclude)hardware-configuration.nix' ':(exclude)local-theme.json')
        local check=1
        if [ "''${1:-}" = --no-check ]; then check=0; shift; fi
        # the weather key and birthdays into secrets.age when they've
        # changed (no passphrase); stops if the repo's copy is newer
        nxsecrets pack --if-changed || { echo "nxpush: not pushed"; return 1; }
        "''${g[@]}" add -A || return
        # nothing new and nothing unpushed: no need to check
        if "''${g[@]}" diff --cached --quiet "''${skip[@]}" &&
           [ "$("''${g[@]}" rev-list --count '@{u}..HEAD' 2>/dev/null)" = 0 ]; then
          echo "nxpush: nothing to push"
          return 0
        fi
        if [ $check = 1 ] && ! nxcheck; then
          echo "nxpush: not pushed (nxpush --no-check to push anyway)"
          return 1
        fi
        if ! "''${g[@]}" diff --cached --quiet "''${skip[@]}"; then
          "''${g[@]}" commit -q -m "''${1:-update}" "''${skip[@]}" || return
        fi
        GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' "''${g[@]}" push
      }
      nxedit() { local out key f; while out=$(cd /etc/nixos && fd -e nix | awk -F/ '{ if (NF==1) printf "0\t\033[1;32m%s\033[0m\n",$0; else { d=$0; sub("/[^/]*$","",d); n=$0; sub(".*/","",n); printf "1\t\033[34m%s/\033[0m\033[39m%s\033[0m\n",d,n } }' | sort | cut -f2- | fzf --ansi --reverse --expect=ctrl-r,ctrl-a --prompt='➜ ' --header 'enter edit · ^r rebuild · ^a git add' --preview 'bat --color=always --theme=ansi --style=numbers /etc/nixos/{}' --preview-window 'right,55%,border-left' --color='gutter:-1,hl:yellow,fg+:-1:bold,bg+:-1,hl+:yellow:bold,pointer:red,marker:green,header:blue,border:8,prompt:blue'); do key=$(head -1 <<< "$out"); f=$(sed -n 2p <<< "$out"); [[ -z "$f" ]] && continue; case "$key" in ctrl-r) nxrebuild ;; ctrl-a) sudo git -C /etc/nixos add -f "$f" && echo "staged $f" ;; *) ''${EDITOR:-nvim} "/etc/nixos/$f" ;; esac; done; }
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
