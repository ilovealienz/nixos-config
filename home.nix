{ pkgs, lib, ... }:
{
  imports = [
    ./home/shell.nix
    ./home/autostart.nix
    ./home/fastfetch.nix
    ./home/local-apps.nix
    ./home/mime.nix
    ./home/secrets.nix
    ./desktop/home.nix
  ];

  home.username = "pc";
  home.homeDirectory = "/home/pc";
  home.stateVersion = "26.05";
  home.packages = [ pkgs.sshfs ];

  programs.git = {
    enable = true;
    settings = {
      user.name = "ilovealienz";
      user.email = "github@brightlittlestars.moe";
    };
  };

  home.activation.mpvConfig = lib.hm.dag.entryAfter ["writeBoundary"] ''
    # offline: skipped, retried next rebuild
    if [ ! -d "$HOME/.config/mpv" ]; then
      run ${pkgs.git}/bin/git clone https://github.com/ilovealienz/my-mpv-config "$HOME/.config/mpv" \
        || echo "mpv config: not cloned (offline?), the next rebuild tries again" >&2
    fi
  '';
}
