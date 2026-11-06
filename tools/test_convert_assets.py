"""Converter unit tests. Skipped when ffmpeg with libvpx-vp9 is missing."""
import importlib.util
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("convert_assets", HERE / "convert-assets.py")
ca = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ca)


def have_vp9():
    if not shutil.which("ffmpeg"):
        return False
    out = subprocess.run(["ffmpeg", "-hide_banner", "-encoders"], capture_output=True, text=True).stdout
    return "libvpx-vp9" in out


class Pure(unittest.TestCase):
    def test_pick_caps_and_keeps_start(self):
        self.assertEqual(ca.pick(list(range(20)), 8)[0], 0)
        self.assertEqual(len(ca.pick(list(range(20)), 8)), 8)
        self.assertEqual(ca.pick([1, 2, 3], 8), [1, 2, 3])

    def test_fit_is_cell_sized_and_transparent_border(self):
        im = Image.new("RGBA", (400, 100), (0, 0, 0, 0))
        im.paste((255, 0, 0, 255), (0, 0, 400, 100))
        out = ca.fit(im, (0, 0, 400, 100))
        self.assertEqual(out.size, (192, 208))
        self.assertEqual(out.getpixel((0, 0))[3], 0)
        self.assertEqual(out.getpixel((96, 104))[3], 255)

    def test_fully_transparent_is_an_error(self):
        with self.assertRaises(RuntimeError):
            ca.union_bbox([Image.new("RGBA", (10, 10), (0, 0, 0, 0))])


@unittest.skipUnless(have_vp9(), "ffmpeg with libvpx-vp9 not available")
class EndToEnd(unittest.TestCase):
    def test_alpha_survives_conversion(self):
        with tempfile.TemporaryDirectory() as t:
            t = Path(t)
            src = t / "in"
            src.mkdir()
            subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                            "color=c=red@1.0:s=320x180:d=1,format=yuva420p,"
                            "drawbox=x=0:y=0:w=80:h=180:color=black@0:t=fill",
                            "-c:v", "libvpx-vp9", "-pix_fmt", "yuva420p", str(src / "a.webm")], check=True)
            mp = t / "map.json"
            mp.write_text('{"idle": {"source": "a.webm"}}')
            out = t / "out"
            self.assertEqual(ca.main(["--input", str(src), "--output", str(out), "--map", str(mp)]), 0)
            frames = sorted((out / "frames" / "idle").glob("*.png"))
            self.assertTrue(1 <= len(frames) <= 8)
            self.assertEqual(Image.open(frames[0]).size, (192, 208))
            self.assertTrue((out / "pet.json").is_file())


if __name__ == "__main__":
    unittest.main()
