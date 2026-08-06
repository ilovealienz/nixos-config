local mod = "SUPER"

-- ── launching ──
hl.bind(mod .. " + Return",    hl.dsp.exec_cmd("kitty"))
hl.bind(mod .. " + R",         hl.dsp.exec_cmd("wmenu-run -f 'Inter 13' -N 24221c -n d4b07b -S e5a440 -s 24221c"))
hl.bind(mod .. " + SHIFT + R", hl.dsp.exec_cmd("fuzzel"))
hl.bind(mod .. " + E",         hl.dsp.exec_cmd("thunar"))
hl.bind(mod .. " + SHIFT + X", hl.dsp.exec_cmd("hyprlock"))

-- ── window management ──
hl.bind(mod .. " + Q",         hl.dsp.window.close())
hl.bind(mod .. " + SHIFT + E", hl.dsp.exit())
hl.bind(mod .. " + F",         hl.dsp.window.fullscreen({ mode = "fullscreen", action = "toggle" }))
hl.bind(mod .. " + V",         hl.dsp.window.float({ action = "toggle" }))
hl.bind(mod .. " + P",         hl.dsp.window.pseudo())
hl.bind(mod .. " + S",         hl.dsp.layout("togglesplit"))

-- ── groups (tabs) ──
hl.bind(mod .. " + G",   hl.dsp.group.toggle())
hl.bind(mod .. " + Tab", hl.dsp.group.next())
-- NOTE: moveintoorcreategroup has no typed lua dispatcher until 0.56.
-- workaround: $mod+G on the target window, then movewindow into it.

-- ── focus / move (vim keys) ──
local dirs = { h = "left", l = "right", k = "up", j = "down" }
for key, dir in pairs(dirs) do
    hl.bind(mod .. " + " .. key,           hl.dsp.focus({ direction = dir }))
    hl.bind(mod .. " + SHIFT + " .. key,   hl.dsp.window.move({ direction = dir }))
end

-- ── resize ──
local resize = { h = { -40, 0 }, l = { 40, 0 }, k = { 0, -40 }, j = { 0, 40 } }
for key, d in pairs(resize) do
    hl.bind(mod .. " + ALT + " .. key,
        hl.dsp.window.resize({ x = d[1], y = d[2], relative = true }),
        { repeating = true })
end

-- ── workspaces ──
for i = 1, 9 do
    hl.bind(mod .. " + " .. i,         hl.dsp.focus({ workspace = i }))
    hl.bind(mod .. " + SHIFT + " .. i, hl.dsp.window.move({ workspace = i }))
end

-- ── screenshots ──
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd("grim -g \"$(slurp)\" - | wl-copy"))
hl.bind("Print",               hl.dsp.exec_cmd("screenshot-menu"))

-- ── media / volume / brightness ──
hl.bind("XF86AudioRaiseVolume",  hl.dsp.exec_cmd("osd-focused --output-volume raise --max-volume 150"), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume",  hl.dsp.exec_cmd("osd-focused --output-volume lower --max-volume 150"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("osd-focused --brightness raise"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("osd-focused --brightness lower"), { locked = true, repeating = true })

hl.bind("XF86AudioMute", hl.dsp.exec_cmd("osd-focused --output-volume mute-toggle"), { locked = true })
hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioNext", hl.dsp.exec_cmd("playerctl next"),       { locked = true })
hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("playerctl previous"),   { locked = true })

-- ── mouse ──
hl.bind(mod .. " + mouse:272", hl.dsp.window.drag())
hl.bind(mod .. " + mouse:273", hl.dsp.window.resize())
