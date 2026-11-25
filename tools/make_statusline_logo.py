#!/usr/bin/env python3
"""Turn one pet frame into coloured terminal text for the Claude Code status line.

    make_statusline_logo.py [--frame PNG] [--rows N] [--out FILE]

Each character carries 2x2 sub-pixels (quadrant block characters, best two-colour fit, 24-bit
colour), so `--rows 16` gives an avatar 32 sub-pixels tall and about 51 wide. Transparent pixels stay transparent, so the
avatar sits on any terminal background. Needs Pillow only (no chafa / ffmpeg at run time);
the output is committed as hooks/statusline-logo.ansi and built into the app (src-tauri/src/statusline.rs).
"""
import argparse
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FRAME = ROOT / "assets/big-blue-fish/frames/idle/00.png"
DEFAULT_OUT = ROOT / "desktop/cross-platform/hooks/statusline-logo.ansi"
OPAQUE = 96  # alpha at or above this counts as a pixel
RESET = "\x1b[0m"


# quadrant glyphs by mask: bit0 top-left, bit1 top-right, bit2 bottom-left, bit3 bottom-right
QUADRANTS = " ▘▝▀▖▌▞▛▗▚▐▜▄▙▟█"
# a character cell is about twice as tall as wide, so a 2x2 sub-pixel is half as wide as it is tall
CELL_ASPECT = 2.0


def to_pixels(frame, rows):
    """Crop to the drawn part and shrink to `rows * 2` sub-pixels tall, wide enough to keep its shape."""
    img = Image.open(frame).convert("RGBA")
    box = img.getchannel("A").getbbox()
    if box:
        img = img.crop(box)
    height = rows * 2
    width = max(2, round(img.width * height / img.height * CELL_ASPECT))
    width += width % 2  # whole cells
    return img.resize((width, height), Image.LANCZOS)


def mean(pixels):
    n = len(pixels)
    return tuple(round(sum(p[i] for p in pixels) / n) for i in range(3))


def distance(pixels, colour):
    return sum(sum((p[i] - colour[i]) ** 2 for i in range(3)) for p in pixels)


def cell(pixels):
    """pixels = [top-left, top-right, bottom-left, bottom-right] as RGBA; returns the coloured glyph."""
    solid = [i for i, p in enumerate(pixels) if p[3] >= OPAQUE]
    if not solid:
        return " "
    if len(solid) < 4:  # edge of the drawing: only the drawn sub-pixels, on the terminal background
        mask = sum(1 << i for i in solid)
        r, g, b = mean([pixels[i] for i in solid])
        return f"\x1b[38;2;{r};{g};{b}m{QUADRANTS[mask]}{RESET}"
    best = None
    for mask in range(1, 8):  # the other 7 splits are these with fg/bg swapped
        top = [pixels[i] for i in range(4) if mask >> i & 1]
        rest = [pixels[i] for i in range(4) if not mask >> i & 1]
        fg, bg = mean(top), mean(rest)
        err = distance(top, fg) + distance(rest, bg)
        if best is None or err < best[0]:
            best = (err, mask, fg, bg)
    _, mask, fg, bg = best
    return f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m\x1b[48;2;{bg[0]};{bg[1]};{bg[2]}m{QUADRANTS[mask]}{RESET}"


def render(img):
    lines = []
    for y in range(0, img.height, 2):
        lines.append(
            "".join(
                cell([img.getpixel((x, y)), img.getpixel((x + 1, y)), img.getpixel((x, y + 1)), img.getpixel((x + 1, y + 1))])
                for x in range(0, img.width, 2)
            )
        )
    return "\n".join(lines) + "\n"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--frame", type=Path, default=DEFAULT_FRAME)
    ap.add_argument("--rows", type=int, default=16, help="text rows (2 sub-pixel rows each)")
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args(argv)
    text = render(to_pixels(args.frame, args.rows))
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({args.rows} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
