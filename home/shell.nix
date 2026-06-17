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
      nxrebuild = "sudo nixos-rebuild switch --flake /etc/nixos#nixos";
      nxupdate = "sudo nix flake update /etc/nixos && nxrebuild";
    };
  };
}
