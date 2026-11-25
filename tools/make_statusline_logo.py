#!/usr/bin/env python3
"""Turn one pet frame into coloured terminal text for the Claude Code status line.

    make_statusline_logo.py [--frame PNG] [--rows N] [--out FILE]

Each text row carries two pixel rows (upper / lower half-block characters with 24-bit colour),
so `--rows 6` gives a 12-pixel-tall avatar. Transparent pixels stay transparent, so the
avatar sits on any terminal background. Needs Pillow only (no chafa / ffmpeg at run time);
the output is committed as hooks/statusline-logo.ansi and printed by hooks/statusline.py.
"""
import argparse
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FRAME = ROOT / "assets/big-blue-fish/frames/idle/00.png"
DEFAULT_OUT = ROOT / "desktop/cross-platform/hooks/statusline-logo.ansi"
OPAQUE = 96  # alpha at or above this counts as a pixel
RESET = "\x1b[0m"


def to_pixels(frame, rows):
    """Crop to the drawn part and shrink so it is `rows * 2` pixels tall."""
    img = Image.open(frame).convert("RGBA")
    box = img.getchannel("A").getbbox()
    if box:
        img = img.crop(box)
    height = rows * 2
    width = max(1, round(img.width * height / img.height))
    return img.resize((width, height), Image.LANCZOS)


def cell(top, bottom):
    def on(p):
        return p[3] >= OPAQUE

    def fg(p):
        return f"\x1b[38;2;{p[0]};{p[1]};{p[2]}m"

    def bg(p):
        return f"\x1b[48;2;{p[0]};{p[1]};{p[2]}m"

    if on(top) and on(bottom):
        return fg(top) + bg(bottom) + "▀" + RESET
    if on(top):
        return fg(top) + "▀" + RESET
    if on(bottom):
        return fg(bottom) + "▄" + RESET
    return " "


def render(img):
    lines = []
    for y in range(0, img.height, 2):
        lines.append("".join(cell(img.getpixel((x, y)), img.getpixel((x, y + 1))) for x in range(img.width)))
    return "\n".join(lines) + "\n"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--frame", type=Path, default=DEFAULT_FRAME)
    ap.add_argument("--rows", type=int, default=6, help="text rows (2 pixel rows each)")
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args(argv)
    text = render(to_pixels(args.frame, args.rows))
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out} ({args.rows} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
