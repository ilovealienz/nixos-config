{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    rustup
    go
    python3
  ];
}
