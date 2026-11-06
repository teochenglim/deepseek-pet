"""Installer tests. Only temp files are touched, never the real ~/.claude or ~/.zcode."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("install_hooks", HERE / "install_hooks.py")
ih = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ih)


class Installer(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        d = Path(self.t.name)
        self.hook = d / "pet-hook.sh"
        self.hook.write_text("#!/bin/sh\n")
        self.cl = d / "claude" / "settings.json"
        self.zc = d / "zcode" / "cli" / "config.json"
        self.zc.parent.mkdir(parents=True)
        self.base = ["--hook-script", str(self.hook), "--claude-settings", str(self.cl), "--zcode-config", str(self.zc)]

    def tearDown(self):
        self.t.cleanup()

    def run_ih(self, action):
        self.assertEqual(ih.main([action, *self.base]), 0)

    def test_install_twice_does_not_duplicate(self):
        self.run_ih("install")
        first = (self.cl.read_text(), self.zc.read_text())
        self.run_ih("install")
        self.assertEqual(first, (self.cl.read_text(), self.zc.read_text()))
        stop = json.loads(self.cl.read_text())["hooks"]["Stop"]
        self.assertEqual(len(stop), 1)

    def test_foreign_hooks_and_settings_survive_install_and_uninstall(self):
        self.cl.parent.mkdir(parents=True)
        self.cl.write_text(json.dumps({"model": "x", "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "other.sh"}]}]}}))
        self.run_ih("install")
        self.run_ih("uninstall")
        data = json.loads(self.cl.read_text())
        self.assertEqual(data["model"], "x")
        self.assertEqual(data["hooks"]["Stop"][0]["hooks"][0]["command"], "other.sh")
        self.assertNotIn("PreToolUse", data["hooks"])

    def test_zcode_gets_enabled_and_its_events(self):
        self.run_ih("install")
        h = json.loads(self.zc.read_text())["hooks"]
        self.assertTrue(h["enabled"])
        self.assertIn("PostToolUseFailure", h["events"])
        self.assertTrue(h["events"]["Stop"][0]["hooks"][0]["command"].endswith("zai # deepseek-pet"))

    def test_invalid_json_is_not_overwritten(self):
        self.cl.parent.mkdir(parents=True)
        self.cl.write_text("{nope")
        with self.assertRaises(SystemExit):
            self.run_ih("install")
        self.assertEqual(self.cl.read_text(), "{nope")

    def test_backup_written_once(self):
        self.cl.parent.mkdir(parents=True)
        self.cl.write_text('{"a": 1}')
        self.run_ih("install")
        self.run_ih("install")
        bak = self.cl.with_name("settings.json.deepseek-pet.bak")
        self.assertEqual(bak.read_text(), '{"a": 1}')


if __name__ == "__main__":
    unittest.main()
