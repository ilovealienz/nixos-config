-- flat, sharp, no animations. desert night palette.

hl.config({
    animations = { enabled = false },

    general = {
        gaps_in     = 2,
        gaps_out    = 2,
        border_size = 2,
        layout      = "dwindle",
        col = {
            active_border   = "rgb(e5a440)",
            inactive_border = "rgb(473f31)",
        },
    },

    decoration = {
        rounding = 0,
        blur   = { enabled = false },
        shadow = { enabled = false },
    },

    dwindle = { preserve_split = true },

    master = { mfact = 0.5 },

    group = {
        col = {
            border_active   = "rgb(e5a440)",
            border_inactive = "rgb(473f31)",
        },
        groupbar = {
            font_family = "Inter",
            font_size   = 11,
            height      = 16,
            gradients   = true,
            gaps_in     = 0,
            gaps_out    = 0,
            text_color  = "rgb(24221c)",
            col = {
                active   = "rgb(e5a440)",
                inactive = "rgb(473f31)",
            },
        },
    },
})
