# yazi's preset theme in the desktop palette: named colours mapped, icon
# colours moved to the palette colour nearest in hue (blue ones to the text colour)
import colorsys, json, re, sys

src, palette_file, out = sys.argv[1:4]
c = json.load(open(palette_file))
text = open(src).read()

named = {
    "blue": c["accent"], "lightblue": c["accent"],
    "cyan": c["accent"], "lightcyan": c["fgBright"],
    "magenta": c["magenta"], "lightmagenta": c["magenta"],
    "yellow": c["yellow"], "lightyellow": c["yellow"],
    "green": c["green"], "lightgreen": c["green"],
    "red": c["red"], "lightred": c["red"],
    "gray": c["surface"], "darkgray": c["muted"],
    "white": c["fgBright"], "black": c["bg"],
}
text = re.sub(r'(fg|bg) = "([a-z]+)"',
              lambda m: f'{m[1]} = "#{named[m[2]]}"' if m[2] in named else m[0], text)

# text on an accent background
acc = f'bg = "#{c["accent"]}"'
text = re.sub(r'\{ ' + re.escape(acc) + r'(?![^}]*\bfg =)', '{ fg = "#' + c["onAccent"] + '", ' + acc, text)


def hsv(h):
    return colorsys.rgb_to_hsv(*(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)))


hues = [(hsv(c[k])[0], c[k]) for k in ("red", "orange", "yellow", "green", "blue", "magenta")]


def nearest(h):
    hh, s, v = hsv(h)
    if s < 0.25 or v < 0.2:
        return c["muted"]
    near = min(hues, key=lambda p: min(abs(p[0] - hh), 1 - abs(p[0] - hh)))[1]
    # blue is the palette's one cool colour; those icons take the text colour
    return c["fg"] if near == c["blue"] else near


head, sep, icons = text.partition("\n[icon]")
icons = re.sub(r'fg = "#([0-9a-fA-F]{6})"', lambda m: f'fg = "#{nearest(m[1])}"', icons)
open(out, "w").write(head + sep + icons)
