"""Lightweight tests for hook scripts."""

import importlib
import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

codex_hook = importlib.import_module('codex_hook')
common = importlib.import_module('common')
setup_hooks = importlib.import_module('setup_hooks')


def norm(path):
    return str(Path(path))


def call_quietly(func, *args, **kwargs):
    with redirect_stdout(StringIO()):
        return func(*args, **kwargs)


class CodexHookTests(unittest.TestCase):
    def test_event_aliases_cover_expected_codex_names(self):
        self.assertEqual(codex_hook.EVENT_ALIASES['post_tool_use'], 'PostToolUse')
        self.assertEqual(codex_hook.EVENT_ALIASES['session_start'], 'SessionStart')
        self.assertEqual(codex_hook.EVENT_ALIASES['stop_failure'], 'StopFailure')


class CommonPathTests(unittest.TestCase):
    def test_load_config_reads_utf8_dialogue_on_windows(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            platform_dir = Path(temp_dir) / 'desktop' / 'cross-platform'
            platform_dir.mkdir(parents=True)
            config_path = platform_dir / 'config.json'
            config_path.write_text(
                json.dumps({
                    'pet_id': 'kotori-minami',
                    'state_map': {
                        'Stop': {'state': 'jumping', 'dialogue': '搞定啦！'}
                    },
                }, ensure_ascii=False),
                encoding='utf-8',
            )

            config, repo_root = common.load_config(str(platform_dir))

            self.assertEqual(repo_root, str(Path(temp_dir).resolve()))
            self.assertEqual(config['state_map']['Stop']['dialogue'], '搞定啦！')

    def test_resolve_base_dir_prefers_pet_base_dir(self):
        self.assertEqual(
            common.resolve_base_dir({'pet_base_dir': 'pets/kotori'}, '/repo'),
            norm('/repo/pets/kotori'),
        )
        self.assertEqual(common.resolve_base_dir({}, '/repo'), norm('/repo'))

    def test_resolve_path_from_base_uses_pet_base_dir_for_relative_sessions_dir(self):
        self.assertEqual(
            common.resolve_path_from_base(
                'runtime/custom-sessions',
                '/repo/pets/kotori',
                'desktop',
                'cross-platform',
                'runtime',
                'sessions',
            ),
            norm('/repo/pets/kotori/runtime/custom-sessions'),
        )
        self.assertEqual(
            common.resolve_path_from_base(
                None,
                '/repo/pets/kotori',
                'desktop',
                'cross-platform',
                'runtime',
                'sessions',
            ),
            norm('/repo/pets/kotori/desktop/cross-platform/runtime/sessions'),
        )


class SetupHooksTests(unittest.TestCase):
    def test_shell_entrypoints_use_lf_line_endings(self):
        platform_dir = SCRIPT_DIR.parent.parent
        shell_files = [
            platform_dir / 'hooks' / 'pet-hook.sh',
            platform_dir / 'scripts' / 'macos' / 'common.sh',
            platform_dir / 'scripts' / 'macos' / 'setup.sh',
            platform_dir / 'scripts' / 'macos' / 'setup-hooks.sh',
            platform_dir / 'scripts' / 'macos' / 'build-and-run.sh',
        ]

        for shell_file in shell_files:
            with self.subTest(path=shell_file):
                data = shell_file.read_bytes()
                self.assertNotIn(b'\r\n', data)

    def test_install_event_hooks_replaces_managed_entries_and_keeps_foreign_ones(self):
        target = setup_hooks.HookTarget(
            name='Codex',
            settings_path='/tmp/hooks.json',
            command='/repo/desktop/cross-platform/hooks/pet-hook.sh codex',
            events=['Stop'],
            executable_names=['codex'],
        )
        settings = {
            'hooks': {
                'Stop': [
                    {'hooks': [{'command': 'echo keep-me', 'type': 'command'}]},
                    {'hooks': [{'command': 'hooks/pet-codex-hook.sh', 'type': 'command'}]},
                    {'hooks': [{'command': target.command, 'type': 'command'}]},
                ]
            }
        }

        call_quietly(setup_hooks.install_event_hooks, settings, target)

        stop_hooks = settings['hooks']['Stop']
        self.assertEqual(len(stop_hooks), 2)
        self.assertEqual(stop_hooks[0]['hooks'][0]['command'], 'echo keep-me')
        self.assertEqual(stop_hooks[1]['hooks'][0]['command'], target.command)

    def test_setup_platform_writes_expected_hook_config(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            settings_path = Path(temp_dir) / 'hooks.json'
            target = setup_hooks.HookTarget(
                name='Claude Code',
                settings_path=str(settings_path),
                command='/repo/desktop/cross-platform/hooks/pet-hook.sh claude-code',
                events=['SessionStart', 'Stop'],
                executable_names=['claude'],
            )

            call_quietly(setup_hooks.setup_platform, target)

            written = json.loads(settings_path.read_text())
            self.assertEqual(set(written['hooks'].keys()), {'SessionStart', 'Stop'})
            for event_name in target.events:
                self.assertEqual(
                    written['hooks'][event_name],
                    [{'hooks': [{'command': target.command, 'type': 'command'}]}],
                )

    def test_setup_platform_writes_codex_windows_command_when_present(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            settings_path = Path(temp_dir) / 'hooks.json'
            target = setup_hooks.HookTarget(
                name='Codex',
                settings_path=str(settings_path),
                command='python3 /mnt/d/repo/codex_hook.py',
                events=['Stop'],
                executable_names=['codex'],
                command_windows=r'python D:\repo\codex_hook.py',
            )

            call_quietly(setup_hooks.setup_platform, target)

            hook = json.loads(settings_path.read_text())['hooks']['Stop'][0]['hooks'][0]
            self.assertEqual(hook['command'], 'python3 /mnt/d/repo/codex_hook.py')
            self.assertEqual(hook['commandWindows'], r'python D:\repo\codex_hook.py')

    def test_build_targets_uses_python_scripts_on_wsl(self):
        platform_dir = Path('/mnt/d/repo/desktop/cross-platform')
        original_is_windows = setup_hooks.is_windows
        original_is_wsl = setup_hooks.is_wsl
        original_detect_python_command = setup_hooks.detect_python_command
        try:
            setup_hooks.is_windows = lambda: False
            setup_hooks.is_wsl = lambda: True
            setup_hooks.detect_python_command = lambda _config=None: 'python3'

            targets, _hooks_config, _python_command = setup_hooks.build_targets(
                platform_dir,
                {'hooks': {}},
            )
        finally:
            setup_hooks.is_windows = original_is_windows
            setup_hooks.is_wsl = original_is_wsl
            setup_hooks.detect_python_command = original_detect_python_command

        commands = {target.name: target.command for target in targets}
        self.assertEqual(
            commands['Claude Code'],
            'python3 /mnt/d/repo/desktop/cross-platform/hooks/scripts/claude_hook.py',
        )
        self.assertEqual(
            commands['Codex'],
            'python3 /mnt/d/repo/desktop/cross-platform/hooks/scripts/codex_hook.py',
        )

    def test_enable_codex_pet_hooks_marks_existing_trusted_entries_enabled(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            settings_path = Path(temp_dir) / 'hooks.json'
            config_toml = Path(temp_dir) / 'config.toml'
            hook_cmd = '/repo/desktop/cross-platform/hooks/pet-hook.sh codex'
            settings = {
                'hooks': {
                    'Stop': [{'hooks': [{'command': hook_cmd, 'type': 'command'}]}]
                }
            }
            settings_path.write_text(json.dumps(settings))

            state_key = setup_hooks.codex_hook_state_key(str(settings_path), 'Stop', 0, 0)
            config_toml.write_text(
                f'[hooks.state."{state_key}"]\ntrusted_hash = "abc123"\n'
            )

            call_quietly(
                setup_hooks.enable_codex_pet_hooks,
                str(settings_path),
                hook_cmd,
                ['Stop'],
            )

            text = config_toml.read_text()
            self.assertIn('enabled = true', text)
            self.assertIn('trusted_hash = "abc123"', text)

    def test_build_command_quotes_windows_style_paths(self):
        command = setup_hooks.build_command([
            r'C:\Program Files\Python\python.exe',
            r'C:\repo with spaces\hooks\scripts\codex_hook.py',
        ])
        self.assertEqual(
            command,
            r'"C:\Program Files\Python\python.exe" "C:\repo with spaces\hooks\scripts\codex_hook.py"',
        )

    def test_config_path_or_default_treats_null_as_auto_detect(self):
        self.assertEqual(
            setup_hooks.config_path_or_default(None, '/default/path'),
            '/default/path',
        )

    def test_detect_python_command_env_overrides_config_value(self):
        original_env = os.environ.get('KOTORI_PET_PYTHON')
        original_validate = setup_hooks.validate_python_command
        try:
            os.environ['KOTORI_PET_PYTHON'] = 'python3'
            setup_hooks.validate_python_command = lambda command: command == 'python3'
            self.assertEqual(
                setup_hooks.detect_python_command('C:/Windows/Python/python.exe'),
                'python3',
            )
        finally:
            setup_hooks.validate_python_command = original_validate
            if original_env is None:
                os.environ.pop('KOTORI_PET_PYTHON', None)
            else:
                os.environ['KOTORI_PET_PYTHON'] = original_env

    def test_should_setup_opencode_honors_skip_env(self):
        original = os.environ.get('KOTORI_PET_SKIP_OPENCODE')
        try:
            os.environ['KOTORI_PET_SKIP_OPENCODE'] = '1'
            self.assertFalse(setup_hooks.should_setup_opencode())
            os.environ['KOTORI_PET_SKIP_OPENCODE'] = 'false'
            self.assertTrue(setup_hooks.should_setup_opencode())
        finally:
            if original is None:
                os.environ.pop('KOTORI_PET_SKIP_OPENCODE', None)
            else:
                os.environ['KOTORI_PET_SKIP_OPENCODE'] = original

    def test_setup_opencode_deploys_plugin_shared_module_and_companion(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            platform_dir = Path(temp_dir) / 'desktop' / 'cross-platform'
            hooks_dir = platform_dir / 'hooks'
            plugins_dir = Path(temp_dir) / 'opencode' / 'plugins'
            hooks_dir.mkdir(parents=True)
            (hooks_dir / 'opencode-plugin.ts').write_text(
                'import "./opencode-shared.mjs";\n',
                encoding='utf-8',
            )
            (hooks_dir / 'opencode-shared.mjs').write_text(
                'export const marker = true;\n',
                encoding='utf-8',
            )

            deployed = call_quietly(
                setup_hooks.setup_opencode,
                platform_dir,
                {'opencode_plugins_dir': str(plugins_dir)},
            )

            self.assertEqual(deployed, str(plugins_dir / 'pet-plugin.ts'))
            self.assertTrue((plugins_dir / 'pet-plugin.ts').exists())
            self.assertTrue((plugins_dir / 'opencode-shared.mjs').exists())
            self.assertEqual(
                (plugins_dir / '.kotori-pet-config-dir').read_text(encoding='utf-8'),
                str(platform_dir.resolve()),
            )


class EndpointTests(unittest.TestCase):
    def test_default_event_endpoint_prefers_explicit_value(self):
        self.assertEqual(
            common.default_event_endpoint({'event_endpoint': 'tcp://127.0.0.1:9999'}),
            'tcp://127.0.0.1:9999',
        )

    def test_default_event_endpoint_uses_socket_path_on_non_windows(self):
        if os.name == 'nt':
            self.skipTest('non-Windows default only')
        self.assertEqual(
            common.default_event_endpoint({'socket_path': '/tmp/custom.sock'}),
            '/tmp/custom.sock',
        )


if __name__ == '__main__':
    unittest.main()


class MappingTests(unittest.TestCase):
    """Animation choices from the menu-bar icon (mapping.json), applied by the hook scripts."""

    def test_ac_0_7_2_hook_uses_mapping_state(self):
        from common import apply_mapping
        info, clip = apply_mapping({'state': 'running', 'dialogue': 'Working...'}, {'state': 'review'})
        self.assertEqual(info, {'state': 'review', 'dialogue': 'Working...'})
        self.assertIsNone(clip)
        info, _ = apply_mapping({'state': 'running', 'dialogue': 'x'}, {'off': True})
        self.assertEqual(info['state'], 'idle')

    def test_ac_0_7_2_unmapped_event_uses_default(self):
        from common import apply_mapping
        default = {'state': 'jumping', 'dialogue': 'Done!'}
        for entry in (None, {}, {'state': 'not-a-state'}, {'clip': ''}, 'junk', {'bogus': 1}):
            info, clip = apply_mapping(default, entry)
            self.assertEqual(info, default, entry)
            self.assertIsNone(clip)

    def test_ac_0_7_2_hook_reads_rust_written_mapping_file(self):
        from common import load_mapping
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'mapping.json'
            path.write_text('{"version": 1, "agents": {"zai": {"Stop": {"clip": "c042"}}}}')
            self.assertEqual(load_mapping(path)['zai']['Stop'], {'clip': 'c042'})
            path.write_text('{broken')
            self.assertEqual(load_mapping(path), {})
            self.assertEqual(load_mapping(Path(d) / 'missing.json'), {})

    def test_ac_0_8_9_default_pool_picks_a_clip_then_waits(self):
        import common
        import random
        with tempfile.TemporaryDirectory() as d:
            stamp = os.path.join(d, '.last')
            entry = {'state': 'running', 'clips': ['c013', 'c054', ' ']}
            first = common.pick_default_clip(entry, stamp, now=1000.0, rng=random.Random(1))
            self.assertIn(first, ('c013', 'c054'))
            # a burst of tool calls must not restart the clip
            self.assertIsNone(common.pick_default_clip(entry, stamp, now=1005.0))
            self.assertIsNotNone(common.pick_default_clip(entry, stamp, now=1000.0 + common.CLIP_GAP_SEC + 1))

    def test_ac_0_8_11_each_source_has_its_own_clip_throttle(self):
        import common
        with tempfile.TemporaryDirectory() as d:
            home = Path(d) / 'home'
            home.mkdir()
            platform = Path(d) / 'platform'
            platform.mkdir()
            sessions = Path(d) / 'sessions'
            (platform / 'config.json').write_text(json.dumps({
                'sessions_dir': str(sessions),
                'event_endpoint': str(Path(d) / 'nobody.sock'),
                'state_map': {'PreToolUse': {'state': 'running', 'dialogue': 'Working...', 'clips': ['c013']}},
            }))
            old = os.environ.get('DEEPSEEK_PET_HOME')
            os.environ['DEEPSEEK_PET_HOME'] = str(home)
            try:
                for source in ('claude-code', 'zai'):
                    common.process_event(str(platform), source, 'PreToolUse', 's-' + source, 'Bash')
            finally:
                if old is None:
                    del os.environ['DEEPSEEK_PET_HOME']
                else:
                    os.environ['DEEPSEEK_PET_HOME'] = old
            stamps = sorted(p.name for p in Path(d).glob('.last-default-clip-*'))
            self.assertEqual(stamps, ['.last-default-clip-claude-code', '.last-default-clip-zai'])

    def test_ac_0_8_9_no_pool_means_no_clip(self):
        import common
        with tempfile.TemporaryDirectory() as d:
            stamp = os.path.join(d, '.last')
            for entry in (None, {}, {'clips': []}, {'clips': 'c013'}, {'clips': [1, None]}, 'junk'):
                self.assertIsNone(common.pick_default_clip(entry, stamp), entry)

    def test_ac_0_7_3_clip_mapping_emits_clip(self):
        from common import apply_mapping
        _, clip = apply_mapping({'state': 'idle', 'dialogue': ''}, {'clip': ' c042 '})
        self.assertEqual(clip, 'c042')

    def test_ac_0_7_3_clip_mapping_keeps_working_state(self):
        from common import apply_mapping
        info, clip = apply_mapping({'state': 'running', 'dialogue': 'Working...'}, {'clip': 'c042'})
        self.assertEqual((info['state'], clip), ('running', 'c042'))
        # a clip replaces the default celebration instead of adding to it
        info, clip = apply_mapping({'state': 'jumping', 'dialogue': 'Done!'}, {'clip': 'c042'})
        self.assertEqual((info['state'], info['dialogue'], clip), ('idle', '', 'c042'))

    def test_ac_0_7_3_process_event_writes_clip_into_payload(self):
        import common
        with tempfile.TemporaryDirectory() as d:
            home = Path(d) / 'home'
            home.mkdir()
            (home / 'mapping.json').write_text('{"agents": {"claude-code": {"Stop": {"clip": "c007"}}}}')
            platform = Path(d) / 'platform'
            platform.mkdir()
            sessions = Path(d) / 'sessions'
            (platform / 'config.json').write_text(json.dumps({
                'sessions_dir': str(sessions),
                'event_endpoint': str(Path(d) / 'nobody.sock'),
                'state_map': {'Stop': {'state': 'jumping', 'dialogue': 'Done!'}},
            }))
            old = os.environ.get('DEEPSEEK_PET_HOME')
            os.environ['DEEPSEEK_PET_HOME'] = str(home)
            try:
                common.process_event(str(platform), 'claude-code', 'Stop', 's1', '')
            finally:
                if old is None:
                    os.environ.pop('DEEPSEEK_PET_HOME', None)
                else:
                    os.environ['DEEPSEEK_PET_HOME'] = old
            saved = json.loads((sessions / 's1.json').read_text())
            self.assertEqual(saved['clip'], 'c007')
            self.assertEqual(saved['state'], 'idle')


class NormaliseTests(unittest.TestCase):
    """Each agent's own event names and payload become the pet's events (agents.json)."""

    def test_ac_0_8_4_gemini_event_names_normalised(self):
        from common import normalize_event
        for raw, want in (('BeforeTool', 'PreToolUse'), ('AfterTool', 'PostToolUse'),
                          ('BeforeAgent', 'UserPromptSubmit'), ('AfterAgent', 'Stop'),
                          ('PreCompress', 'PreCompact'), ('SessionStart', 'SessionStart')):
            self.assertEqual(normalize_event('gemini', raw), want, raw)
        self.assertEqual(normalize_event('gemini', 'SomethingNew'), 'SomethingNew', 'unknown names pass through')
        self.assertEqual(normalize_event('gemini', ''), '')

    def test_ac_0_8_4_copilot_camelcase_payload_normalised(self):
        import generic_hook
        from common import normalize_event
        for raw, want in (('sessionStart', 'SessionStart'), ('userPromptSubmitted', 'UserPromptSubmit'),
                          ('preToolUse', 'PreToolUse'), ('postToolUse', 'PostToolUse'),
                          ('agentStop', 'Stop'), ('errorOccurred', 'StopFailure'), ('sessionEnd', 'SessionEnd')):
            self.assertEqual(normalize_event('copilot', raw), want, raw)
        # Copilot's payload has no event name: it arrives as the command's argument; toolName is camelCase
        self.assertEqual(generic_hook.pick_event('copilot', {'toolName': 'bash'}, 'preToolUse'), 'PreToolUse')
        a = generic_hook.session_id_for('copilot', {}, '/work/a')
        self.assertEqual(a, generic_hook.session_id_for('copilot', {}, '/work/a'), 'stable per folder')
        self.assertNotEqual(a, generic_hook.session_id_for('copilot', {}, '/work/b'))

    def test_ac_0_8_4_cursor_stop_status_normalised(self):
        import generic_hook
        self.assertEqual(generic_hook.pick_event('cursor', {'hook_event_name': 'stop', 'status': 'completed'}, ''), 'Stop')
        self.assertEqual(generic_hook.pick_event('cursor', {'hook_event_name': 'stop', 'status': 'error'}, ''), 'StopFailure')
        self.assertEqual(generic_hook.pick_event('cursor', {'hook_event_name': 'beforeSubmitPrompt'}, ''), 'UserPromptSubmit')

    def test_ac_0_8_4_every_listed_agent_has_known_events(self):
        from common import load_agents, normalize_event
        agents = load_agents()
        self.assertGreaterEqual(len(agents), 10)
        for agent in agents:
            for event in agent['events']:
                native = agent.get('native', {}).get(event, event)
                self.assertEqual(normalize_event(agent['id'], native), event, (agent['id'], event))


class DefaultAnimationTests(unittest.TestCase):
    """Out of the box the pet should show what the agent is doing (0.8.0 defaults)."""

    def setUp(self):
        cfg = json.loads((SCRIPT_DIR.parent.parent / 'config.example.json').read_text())
        self.state_map = cfg['state_map']

    def state(self, event):
        return common.resolve_state(event, self.state_map)['state']

    def test_ac_0_8_8_compact_shows_thinking_then_work(self):
        self.assertEqual(self.state('PreCompact'), 'review', 'compacting is thinking, not waiting for the user')
        self.assertEqual(self.state('PostCompact'), 'running')

    def test_ac_0_8_8_prompt_is_thinking_tools_are_working(self):
        self.assertEqual(self.state('UserPromptSubmit'), 'review')
        self.assertEqual(self.state('PreToolUse'), 'running')
        self.assertEqual(self.state('PostToolUse'), 'running')

    def test_ac_0_8_8_subagent_stop_does_not_idle_a_busy_agent(self):
        self.assertEqual(self.state('SubagentStop'), 'running')

    def test_ac_0_8_8_only_the_user_blocks_with_waiting(self):
        waiting = sorted(e for e, v in self.state_map.items() if v['state'] == 'waiting')
        self.assertEqual(waiting, ['PermissionRequest', 'QuestionAsked'])
