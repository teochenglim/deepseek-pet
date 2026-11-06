"""Installer tests. Only temp folders are touched, never the real ~/.claude, ~/.zcode and friends."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("install_hooks", HERE / "install_hooks.py")
ih = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ih)


class Base(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.home = Path(self.t.name)
        hooks = self.home / "app" / "hooks"
        hooks.mkdir(parents=True)
        self.hook = hooks / "pet-hook.sh"
        self.hook.write_text("#!/bin/sh\n")
        (hooks / "opencode-plugin.ts").write_text("export const PetPlugin = () => ({})\n")

    def tearDown(self):
        self.t.cleanup()

    def run_ih(self, action, *extra):
        self.assertEqual(ih.main([action, "--hook-script", str(self.hook), "--home", str(self.home), *extra]), 0)

    def path(self, rel):
        return self.home / rel

    def mk(self, *dirs):
        for d in dirs:
            (self.home / d).mkdir(parents=True, exist_ok=True)


class Installer(Base):
    """Claude Code and ZCode (0.3.0 behaviour, now driven by agents.json)."""

    def setUp(self):
        super().setUp()
        self.mk(".claude", ".zcode/cli")
        self.cl = self.path(".claude/settings.json")
        self.zc = self.path(".zcode/cli/config.json")

    def test_install_twice_does_not_duplicate(self):
        self.run_ih("install")
        first = (self.cl.read_text(), self.zc.read_text())
        self.run_ih("install")
        self.assertEqual(first, (self.cl.read_text(), self.zc.read_text()))
        self.assertEqual(len(json.loads(self.cl.read_text())["hooks"]["Stop"]), 1)

    def test_foreign_hooks_and_settings_survive_install_and_uninstall(self):
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
        self.cl.write_text("{nope")
        with self.assertRaises(SystemExit):
            self.run_ih("install")
        self.assertEqual(self.cl.read_text(), "{nope")

    def test_backup_written_once(self):
        self.cl.write_text('{"a": 1}')
        self.run_ih("install")
        self.run_ih("install")
        self.assertEqual(self.cl.with_name("settings.json.deepseek-pet.bak").read_text(), '{"a": 1}')

    def test_claude_gets_post_compact(self):
        self.run_ih("install", "--only", "claude-code")
        self.assertIn("PostCompact", json.loads(self.cl.read_text())["hooks"])


class MoreAgents(Base):
    def test_ac_0_8_1_gemini_nested_hooks_format(self):
        self.mk(".gemini")
        self.path(".gemini/settings.json").write_text('{"theme": "dark"}')
        self.run_ih("install", "--only", "gemini")
        data = json.loads(self.path(".gemini/settings.json").read_text())
        self.assertEqual(data["theme"], "dark")
        for name in ("BeforeTool", "AfterTool", "BeforeAgent", "AfterAgent", "SessionStart", "SessionEnd"):
            entry = data["hooks"][name][0]["hooks"][0]
            self.assertEqual(entry["type"], "command")
            self.assertTrue(entry["command"].endswith("gemini # deepseek-pet"), entry)
        self.assertNotIn("PreToolUse", data["hooks"], "Gemini uses its own event names")

    def test_ac_0_8_1_qwen_settings_hooks_format(self):
        self.mk(".qwen")
        self.run_ih("install", "--only", "qwen")
        data = json.loads(self.path(".qwen/settings.json").read_text())
        self.assertIn("UserPromptSubmit", data["hooks"])
        self.assertTrue(data["hooks"]["Stop"][0]["hooks"][0]["command"].endswith("qwen # deepseek-pet"))

    def test_ac_0_8_2_codex_hooks_json(self):
        self.mk(".codex")
        self.run_ih("install", "--only", "codex")
        data = json.loads(self.path(".codex/hooks.json").read_text())
        self.assertEqual(set(data["hooks"]), {"SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PermissionRequest", "Stop"})
        self.assertTrue(data["hooks"]["Stop"][0]["hooks"][0]["command"].endswith("codex # deepseek-pet"))

    def test_ac_0_8_2_kimi_toml_hooks_entries(self):
        self.mk(".kimi-code")
        cfg = self.path(".kimi-code/config.toml")
        cfg.write_text('# my settings\nmodel = "k2"\n')
        self.run_ih("install", "--only", "kimi")
        text = cfg.read_text()
        self.assertIn('# my settings\nmodel = "k2"', text)
        self.assertIn("[[hooks]]", text)
        blocks = text.split("[[hooks]]")[1:]
        for block in blocks:
            keys = {line.split("=")[0].strip() for line in block.splitlines() if "=" in line}
            self.assertEqual(keys, {"event", "matcher", "command", "timeout"}, "any other field breaks Kimi's loader")
        self.run_ih("install", "--only", "kimi")
        self.assertEqual(cfg.read_text(), text, "second install changes nothing")

    def test_ac_0_8_2_copilot_hooks_file(self):
        self.mk(".copilot")
        self.run_ih("install", "--only", "copilot")
        data = json.loads(self.path(".copilot/hooks/deepseek-pet.json").read_text())
        self.assertEqual(data["version"], 1)
        entry = data["hooks"]["preToolUse"][0]
        self.assertEqual(entry["type"], "command")
        self.assertTrue(entry["bash"].endswith("copilot preToolUse # deepseek-pet"), entry)

    def test_ac_0_8_2_cursor_hooks_json_version_1(self):
        self.mk(".cursor")
        self.run_ih("install", "--only", "cursor")
        data = json.loads(self.path(".cursor/hooks.json").read_text())
        self.assertEqual(data["version"], 1)
        self.assertTrue(data["hooks"]["stop"][0]["command"].endswith("cursor stop # deepseek-pet"))

    def test_ac_0_8_2_opencode_plugin_copied(self):
        self.mk(".config/opencode")
        self.run_ih("install", "--only", "opencode")
        plugin = self.path(".config/opencode/plugins/deepseek-pet.ts")
        text = plugin.read_text()
        self.assertIn("opencode-plugin.ts", text)
        self.assertTrue(text.startswith("# deepseek-pet"))
        self.assertTrue((self.hook.parent / ".kotori-pet-config-dir").is_file())

    def test_ac_0_8_3_install_all_twice_no_duplicates(self):
        self.mk(".claude", ".zcode/cli", ".codex", ".gemini", ".qwen", ".kimi-code", ".copilot", ".cursor", ".config/opencode")
        self.run_ih("install")
        def snapshot():
            return {str(p.relative_to(self.home)): p.read_text() for p in sorted(self.home.rglob("*"))
                    if p.is_file() and p.suffix in (".json", ".toml", ".ts") and "app" not in p.parts}
        first = snapshot()
        self.assertGreaterEqual(len(first), 9)
        self.run_ih("install")
        self.assertEqual(first, snapshot())

    def test_ac_0_8_3_uninstall_all_keeps_foreign_hooks(self):
        self.mk(".claude", ".gemini", ".cursor", ".kimi-code", ".copilot")
        self.path(".gemini/settings.json").write_text(json.dumps({"hooks": {"BeforeTool": [{"hooks": [{"type": "command", "command": "mine.sh"}]}]}}))
        self.path(".cursor/hooks.json").write_text(json.dumps({"version": 1, "hooks": {"stop": [{"command": "mine.sh"}]}}))
        self.path(".kimi-code/config.toml").write_text('model = "k2"\n\n[[hooks]]\nevent = "Stop"\nmatcher = ""\ncommand = "mine.sh"\ntimeout = 5\n')
        self.run_ih("install")
        self.run_ih("uninstall")
        g = json.loads(self.path(".gemini/settings.json").read_text())
        self.assertEqual(g["hooks"]["BeforeTool"][0]["hooks"][0]["command"], "mine.sh")
        self.assertEqual(list(g["hooks"]), ["BeforeTool"])
        c = json.loads(self.path(".cursor/hooks.json").read_text())
        self.assertEqual(c["hooks"], {"stop": [{"command": "mine.sh"}]})
        toml = self.path(".kimi-code/config.toml").read_text()
        self.assertIn('command = "mine.sh"', toml)
        self.assertNotIn("deepseek-pet", toml)
        self.assertFalse(self.path(".copilot/hooks/deepseek-pet.json").exists())

    def test_status_reports_present_and_installed(self):
        self.mk(".gemini", ".cursor")
        self.run_ih("install", "--only", "gemini")
        st = ih.status(ih.load_agents(), self.home, {})
        self.assertEqual((st["gemini"]["present"], st["gemini"]["installed"]), (True, True))
        self.assertEqual((st["cursor"]["present"], st["cursor"]["installed"]), (True, False))
        self.assertEqual((st["qwen"]["present"], st["qwen"]["installed"]), (False, False))
        self.assertNotIn("deepseek-harness", st)

    def test_install_skips_agents_that_are_not_on_this_mac(self):
        self.run_ih("install")
        self.assertFalse(self.path(".gemini").exists())

    def test_unknown_agent_is_rejected(self):
        with self.assertRaises(SystemExit):
            self.run_ih("install", "--only", "nope")


if __name__ == "__main__":
    unittest.main()
