"""Read one captured game frame: where the route arrow is, and whether the car moves.

Two cues, both from the rendered canvas rather than from the page's API:

- the route arrow: a saturated cyan glow drawn over the world, found as clusters
  of pixels where blue dominates red and the value is high;
- motion: the mean absolute difference of the bottom band (road under the
  camera) between consecutive frames, which is high when driving and near zero
  when parked.

Usage: analyze_frame.py <frame> [--json] [--prev <previous frame>]
"""

import json
import sys

from PIL import Image

CYAN_MIN_B = 150
CYAN_DOMINANCE = 45
CYAN_MIN_G = 90
MIN_PIXELS = 120


def cyan_mask(px, w, h):
    mask = []
    for y in range(h):
        row = []
        for x in range(w):
            r, g, b = px[x, y]
            row.append(1 if (b >= CYAN_MIN_B and b - r >= CYAN_DOMINANCE and g >= CYAN_MIN_G) else 0)
        mask.append(row)
    return mask


def clusters(mask, w, h, min_pixels=60):
    seen = [[0] * w for _ in range(h)]
    found = []
    for y in range(h):
        for x in range(w):
            if not mask[y][x] or seen[y][x]:
                continue
            stack = [(x, y)]
            seen[y][x] = 1
            count = 0
            x0 = x1 = x
            y0 = y1 = y
            while stack:
                cx, cy = stack.pop()
                count += 1
                x0, x1 = min(x0, cx), max(x1, cx)
                y0, y1 = min(y0, cy), max(y1, cy)
                for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                    if 0 <= nx < w and 0 <= ny < h and mask[ny][nx] and not seen[ny][nx]:
                        seen[ny][nx] = 1
                        stack.append((nx, ny))
            if count >= min_pixels:
                found.append({"pixels": count, "box": [x0, y0, x1, y1], "cx": (x0 + x1) / 2 / w, "cy": (y0 + y1) / 2 / h})
    found.sort(key=lambda c: -c["pixels"])
    return found


def band_diff(prev_path, cur_path, band=(0.72, 1.0)):
    """Mean absolute difference in the bottom strip of two frames."""
    a = Image.open(prev_path).convert("L").resize((247, 186))
    b = Image.open(cur_path).convert("L").resize((247, 186))
    w, h = a.size
    y0 = int(h * band[0])
    da = a.crop((0, y0, w, h)).tobytes()
    db = b.crop((0, y0, w, h)).tobytes()
    if len(da) != len(db):
        return None
    return sum(abs(x - y) for x, y in zip(da, db)) / len(da)


def main():
    path = sys.argv[1]
    as_json = "--json" in sys.argv
    prev = sys.argv[sys.argv.index("--prev") + 1] if "--prev" in sys.argv else None

    im = Image.open(path).convert("RGB")
    w, h = im.size
    sw, sh = 494, 372
    small = im.resize((sw, sh))
    px = small.load()
    arrows = clusters(cyan_mask(px, sw, sh), sw, sh)

    # Impostors: the minimap corner is cyan and never moves with the car, and
    # neon/signage hugs the frame edge or sits low.
    world = [
        a
        for a in arrows
        if a["pixels"] >= MIN_PIXELS
        and 0.05 < a["cx"] < 0.95
        and a["cy"] < 0.75
        and not (a["cx"] > 0.72 and a["cy"] > 0.62)
    ]
    target = min(world, key=lambda a: a["cy"]) if world else None

    report = {
        "frame": path,
        "size": [w, h],
        "arrow": target,
        "arrowCandidates": arrows[:3],
        "arrowVisible": bool(world),
        "bandDiff": band_diff(prev, path) if prev else None,
    }
    print(json.dumps(report) if as_json else json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
