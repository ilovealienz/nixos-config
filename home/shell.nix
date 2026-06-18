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
      nxrebuild = "sudo git -C /etc/nixos add -f hardware-configuration.nix && sudo nixos-rebuild switch --flake /etc/nixos#nixos";
      nxupdate = "cd /etc/nixos && sudo nix flake update && nxrebuild";
      nxpush = "GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' sudo -E git -C /etc/nixos add -f . && sudo git -C /etc/nixos rm --cached hardware-configuration.nix 2>/dev/null; GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' sudo -E git -C /etc/nixos commit -m 'update' && GIT_SSH_COMMAND='ssh -i /home/pc/.ssh/id_ed25519' sudo -E git -C /etc/nixos push";
    };
    initExtra = ''
      nxrun() { nix run nixpkgs#"$1" }
    '';
  };
}
