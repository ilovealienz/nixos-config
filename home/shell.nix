{ ... }:
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
      # ── prompt (like robbyrussell) ──
      setopt PROMPT_SUBST

      git_prompt() {
        local ref dirty
        ref=$(git symbolic-ref --short HEAD 2>/dev/null) || return
        [[ -n $(git status --porcelain 2>/dev/null) ]] && dirty=' %F{yellow}✗%f'
        echo "%F{blue}git:(%F{red}''${ref}%F{blue})%f''${dirty}"
      }

      PROMPT='%(?:%F{green}➜:%F{red}➜)%f %F{cyan}%c%f $(git_prompt) '

      [[ -f ~/.aliases ]] && source ~/.aliases
    '';
  };
}
