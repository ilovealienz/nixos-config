-- pc: 4K above, two 1080p below. DP-1 is the main/centre display.

hl.monitor({ output = "DP-1",     mode = "1920x1080@143.86", position = "1920x1440", scale = 1 })
hl.monitor({ output = "DP-2",     mode = "1920x1080@165",    position = "0x1440",    scale = 1 })
hl.monitor({ output = "HDMI-A-2", mode = "2560x1440@59.95",  position = "1600x0",    scale = 1 })

-- pin workspaces so displays come up correctly regardless of init order
hl.workspace_rule({ workspace = "1", monitor = "DP-1",     default = true })
hl.workspace_rule({ workspace = "2", monitor = "DP-2",     default = true, layoutname = "master" })
hl.workspace_rule({ workspace = "3", monitor = "HDMI-A-2", default = true })
hl.workspace_rule({ workspace = "4", monitor = "DP-1" })
hl.workspace_rule({ workspace = "5", monitor = "DP-1" })
