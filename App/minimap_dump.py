"""Dump the minimap corner of a captured frame as ASCII, to see whether the road
geometry is legible enough to steer by.

Two masks: cyan (the same family as the route arrow) and warm/orange (the roads
in this game's minimap). Prints a coarse grid so the shape can be read as text.
"""

import sys

from PIL import Image

path = sys.argv[1]
im = Image.open(path).convert("RGB")
W, H = im.size
# Bottom-LEFT quadrant: the circular minimap lives there (the cyan ring in the
# bottom-right corner is the speedometer's nitrous arc, not a map).
crop = im.crop((0, int(H * 0.55), int(W * 0.34), H))
cw, ch = crop.size
cols, rows = 56, 30
small = crop.resize((cols, rows))
raw = small.tobytes()  # RGB triples, row-major

print(f"frame {W}x{H} | minimap crop {cw}x{ch} -> {cols}x{rows}")
for y in range(rows):
    line = ""
    for x in range(cols):
        i = (y * cols + x) * 3
        r, g, b = raw[i], raw[i + 1], raw[i + 2]
        if b >= 140 and b - r >= 40 and g >= 80:
            line += "C"  # cyan
        elif r >= 110 and g >= 60 and b <= 95 and r - b >= 35:
            line += "o"  # warm road
        elif max(r, g, b) < 45:
            line += "."
        else:
            line += " "
    print(f"{y:02d} {line}")
