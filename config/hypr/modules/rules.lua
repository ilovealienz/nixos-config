-- ── layer rules ──
hl.layer_rule({ match = { namespace = "waybar" }, ignore_alpha = 0 })

-- ── floating popups ──
hl.window_rule({ match = { class = "kitty-float" }, float = true, size = "900 600", center = true })
hl.window_rule({ match = { class = "pavucontrol" },      float = true })
hl.window_rule({ match = { class = "blueman-manager" },  float = true })

-- ── app → workspace ──
local ws = {
    ["1"] = { "firefox", "brave-browser", "floorp" },
    ["2"] = { "spotify", "signal", "vesktop" },
    ["3"] = { "mpv" },
    ["4"] = { "kitty" },
    ["5"] = { "(?i)thunar" },
    ["6"] = { "org.qbittorrent.qBittorrent" },
    ["7"] = { ".virt-manager-wrapped" },
}

for workspace, classes in pairs(ws) do
    for _, c in ipairs(classes) do
        hl.window_rule({ match = { class = c }, workspace = workspace })
    end
end
