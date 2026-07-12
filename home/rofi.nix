{ ... }:

{
  home.file.".config/rofi/emoji-dark.rasi".text = ''
    * {
        bg:        #1a1a1aE6;
        bg-alt:    #2a2a2aE6;
        fg:        #e0e0e0FF;
        fg-dim:    #808080FF;
        accent:    #3a3a3aFF;
        accent-fg: #ffffffFF;

        background-color: transparent;
        text-color:        @fg;

        margin:  0px;
        padding: 0px;
        spacing: 0px;
    }

    window {
        background-color: @bg;
        border:            0px;
        border-radius:      12px;
        width:              480px;
        location:           center;
        anchor:             center;
        x-offset:           0px;
        y-offset:           0px;
        padding:            14px;
    }

    mainbox {
        children: [ inputbar, listview ];
        spacing:  10px;
    }

    inputbar {
        background-color: @bg-alt;
        border-radius:     8px;
        padding:           10px 12px;
        children:          [ prompt, entry ];
    }

    prompt {
        text-color: @fg-dim;
        padding:    0px 8px 0px 0px;
    }

    entry {
        text-color:        @fg;
        placeholder:       "Search emoji…";
        placeholder-color: @fg-dim;
    }

    listview {
        lines:        8;
        columns:      1;
        spacing:      3px;
        fixed-height: false;
        scrollbar:    false;
    }

    element {
        padding:       8px 10px;
        border-radius: 6px;
    }

    element normal.normal {
        background-color: transparent;
        text-color:         @fg;
    }

    element selected.normal {
        background-color: @accent;
        text-color:         @accent-fg;
    }

    element-icon {
        size: 1.2em;
    }

    element-text {
        vertical-align: 0.5;
    }
  '';
}
