-- all of these are writeShellScriptBin wrappers or packages in PATH,
-- so no /nix/store paths get baked into this file.

hl.on("hyprland.start", function()
    hl.exec_cmd("hypr-dbus-env")
    hl.exec_cmd("waybar")
    hl.exec_cmd("polkit-agent")
    hl.exec_cmd("set-wallpaper")
end)
