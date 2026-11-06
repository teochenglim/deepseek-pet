#!/usr/bin/env python3
"""Convert Big Blue Fish VP9-alpha WebM clips into the renderer's per-state PNG frames.

Output layout (what desktop/cross-platform/src/animator.js loads):
    <output>/frames/<state>/NN.png     192x208 RGBA, at most 8 frames per state
    <output>/frames/frames-manifest.json   row per state (merged, other rows kept)
    <output>/pet.json                  source file, frame count, original resolution

Needs ffmpeg/ffprobe with libvpx-vp9 (the native vp9 decoder drops alpha) and Pillow.
"""
import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path

from PIL import Image

CELL_W, CELL_H = 192, 208
MAX_FRAMES = 8
SAMPLE_FPS = 12
ALPHA_MIN = 8
EXTRA_FRAMES = 16      # frames per extra clip (kept small: 106 clips ship in the app)
EXTRA_FPS = 8
EXTRA_COLORS = 128     # palette size; about 11 KB per frame instead of 50 KB
EXTRA_SCALE = "640:360"  # decode small, the cell is only 192x208


def run(cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def probe_size(path):
    out = run(["ffprobe", "-v", "error", "-select_streams", "v:0",
               "-show_entries", "stream=width,height", "-of", "csv=p=0", str(path)])
    w, h = out.strip().split(",")
    return int(w), int(h)


def decode(path, tmp):
    """Decode every SAMPLE_FPS-th frame to RGBA PNG, keeping alpha (libvpx-vp9 before -i)."""
    run(["ffmpeg", "-v", "error", "-y", "-c:v", "libvpx-vp9", "-i", str(path),
         "-vf", f"fps={SAMPLE_FPS}", "-pix_fmt", "rgba", str(tmp / "%04d.png")])
    frames = sorted(tmp.glob("*.png"))
    if not frames:
        raise RuntimeError(f"no frames decoded from {path}")
    return frames


def pick(items, n):
    """n evenly spaced items, keeping the loop start."""
    if len(items) <= n:
        return items
    return [items[(i * len(items)) // n] for i in range(n)]


def union_bbox(images):
    box = None
    for im in images:
        b = im.getchannel("A").point(lambda a: 255 if a > ALPHA_MIN else 0).getbbox()
        if b is None:
            continue
        box = b if box is None else (min(box[0], b[0]), min(box[1], b[1]),
                                     max(box[2], b[2]), max(box[3], b[3]))
    if box is None:
        raise RuntimeError("clip is fully transparent (alpha lost?)")
    return box


def fit(im, box):
    """Crop to the shared box, scale proportionally into the cell, center on transparency."""
    im = im.crop(box)
    r = min(CELL_W / im.width, CELL_H / im.height)
    im = im.resize((max(1, round(im.width * r)), max(1, round(im.height * r))), Image.LANCZOS)
    cell = Image.new("RGBA", (CELL_W, CELL_H), (0, 0, 0, 0))
    cell.paste(im, ((CELL_W - im.width) // 2, (CELL_H - im.height) // 2), im)
    return cell


def find_source(directory, name):
    """Match a file name ignoring Unicode normalisation (macOS stores NFD)."""
    want = unicodedata.normalize("NFC", name)
    for p in directory.glob("*.webm"):
        if unicodedata.normalize("NFC", p.name) == want:
            return p
    raise FileNotFoundError(f"{name} not found in {directory}")


def convert_state(state, src, out_frames, max_frames, flip=False):
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        chosen = pick(decode(src, tmp), max_frames)
        images = [Image.open(p).convert("RGBA") for p in chosen]
        box = union_bbox(images)
        d = out_frames / state
        d.mkdir(parents=True, exist_ok=True)
        for old in d.glob("*.png"):
            old.unlink()
        names = []
        for i, im in enumerate(images):
            name = f"{i:02d}.png"
            cell = fit(im, box)
            if flip:
                cell = cell.transpose(Image.FLIP_LEFT_RIGHT)
            cell.save(d / name)
            names.append(name)
    return names


def merge_manifest(path, rows):
    manifest = json.loads(path.read_text()) if path.exists() else {"ok": True, "rows": []}
    existing = {r["state"]: r for r in manifest["rows"]}
    for state, names in rows.items():
        existing[state] = {"state": state, "frames": names, "method": "ffmpeg"}
    order = [r["state"] for r in manifest["rows"]] + [s for s in rows if s not in {r["state"] for r in manifest["rows"]}]
    manifest["rows"] = [existing[s] for s in order]
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")


def clip_duration(path):
    out = run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)])
    return float(out.strip())


def decode_n(path, tmp, n, duration):
    """Decode about n evenly spaced frames, scaled down, alpha kept."""
    run(["ffmpeg", "-v", "error", "-y", "-c:v", "libvpx-vp9", "-i", str(path),
         "-vf", f"fps={n / duration:.5f},scale={EXTRA_SCALE}:flags=lanczos",
         "-pix_fmt", "rgba", str(tmp / "%04d.png")])
    frames = sorted(tmp.glob("*.png"))[:n]
    if not frames:
        raise RuntimeError(f"no frames decoded from {path}")
    return frames


def clip_id(index):
    return f"c{index:03d}"


def convert_extras(input_dir, out_frames, titles, n=EXTRA_FRAMES):
    """Every clip becomes frames/extras/<id>/NN.png (palette PNG) plus extras/catalog.json."""
    root = out_frames / "extras"
    root.mkdir(parents=True, exist_ok=True)
    clips = sorted(input_dir.glob("*.webm"), key=lambda p: unicodedata.normalize("NFC", p.name))
    catalog = []
    for i, src in enumerate(clips, start=1):
        zh = unicodedata.normalize("NFC", src.stem)
        en, category = titles.get(zh, [zh, "Moods and movement"])
        cid = clip_id(i)
        duration = clip_duration(src)
        with tempfile.TemporaryDirectory() as t:
            images = [Image.open(p).convert("RGBA") for p in decode_n(src, Path(t), n, duration)]
            box = union_bbox(images)
            d = root / cid
            d.mkdir(parents=True, exist_ok=True)
            for old in d.glob("*.png"):
                old.unlink()
            for j, im in enumerate(images):
                cell = fit(im, box)
                cell.quantize(colors=EXTRA_COLORS, method=Image.Quantize.FASTOCTREE,
                              dither=Image.Dither.NONE).save(d / f"{j:02d}.png", optimize=True)
        catalog.append({"id": cid, "title": en, "title_zh": zh, "category": category,
                        "frames": len(images), "fps": EXTRA_FPS, "source": src.name,
                        "duration_sec": round(duration, 2)})
        print(f"{cid} {en}: {len(images)} frames")
    (root / "catalog.json").write_text(json.dumps({"version": 1, "clips": catalog}, indent=1, ensure_ascii=False) + "\n")
    return catalog


def main(argv=None):
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--input", required=True, type=Path, help="directory holding the .webm files")
    ap.add_argument("--output", required=True, type=Path, help="e.g. assets/big-blue-fish")
    ap.add_argument("--map", type=Path, default=here / "state-map.json")
    ap.add_argument("--states", nargs="*", help="subset of states (default: all in the map)")
    ap.add_argument("--max-frames", type=int, default=MAX_FRAMES)
    ap.add_argument("--extras", action="store_true", help="convert every clip into frames/extras/ for the Play animation menu")
    ap.add_argument("--titles", type=Path, default=here / "clip-titles.json")
    args = ap.parse_args(argv)

    if args.max_frames > MAX_FRAMES:
        ap.error(f"--max-frames cannot exceed {MAX_FRAMES} (renderer column limit)")
    if args.extras:
        titles = json.loads(args.titles.read_text())
        convert_extras(args.input, args.output / "frames", titles)
        return 0
    state_map = json.loads(args.map.read_text())
    states = args.states or list(state_map)
    out_frames = args.output / "frames"
    rows, mapped = {}, {}
    for state in states:
        src = find_source(args.input, state_map[state]["source"])
        rows[state] = convert_state(state, src, out_frames, args.max_frames,
                                    flip=bool(state_map[state].get("flip")))
        w, h = probe_size(src)
        mapped[state] = {"source": state_map[state]["source"], "frames": len(rows[state]),
                         "original_resolution": [w, h], "flip": bool(state_map[state].get("flip")),
                         "note": state_map[state].get("note", "")}
        print(f"{state}: {len(rows[state])} frames from {src.name} ({w}x{h})")

    merge_manifest(out_frames / "frames-manifest.json", rows)
    pet_path = args.output / "pet.json"
    prev = json.loads(pet_path.read_text()) if pet_path.exists() else {}
    all_mapped = {**prev.get("mapped", {}), **mapped}
    used = {unicodedata.normalize("NFC", v["source"]) for v in all_mapped.values()}
    unmapped = sorted(unicodedata.normalize("NFC", p.name) for p in args.input.glob("*.webm")
                      if unicodedata.normalize("NFC", p.name) not in used)
    pet = {"version": 1, "cell": [CELL_W, CELL_H], "max_frames": MAX_FRAMES, "sample_fps": SAMPLE_FPS,
           "mapped": all_mapped, "unmapped": unmapped}
    pet_path.write_text(json.dumps(pet, indent=2, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
