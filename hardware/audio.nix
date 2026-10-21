{ ... }:
{
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  hardware.alsa.enablePersistence = true;   # keep mixer levels across reboots
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };
}
