//! AC0.8.x gates: installing hooks for more agents and their event names. The behaviour is tested
//! by the Python cases in tools/test_install_hooks.py and hooks/scripts/test_hooks.py;
//! each is run here by name so `make ac-map` can trace it.
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn py_case(file: &str, name: &str) {
    let out = Command::new("python3")
        .current_dir(root())
        .args(["-m", "unittest", file, "-k", name])
        .output()
        .expect("python3 is required to run the hook and installer tests");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "{name} failed:\n{text}");
    assert!(
        text.contains("Ran 1 test"),
        "{name} matched no single test:\n{text}"
    );
}

#[test]
fn ac_0_8_1_gemini_nested_hooks_format() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_1_gemini_nested_hooks_format",
    );
}

#[test]
fn ac_0_8_1_qwen_settings_hooks_format() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_1_qwen_settings_hooks_format",
    );
}

#[test]
fn ac_0_8_2_codex_hooks_json() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_2_codex_hooks_json",
    );
}

#[test]
fn ac_0_8_2_kimi_toml_hooks_entries() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_2_kimi_toml_hooks_entries",
    );
}

#[test]
fn ac_0_8_2_copilot_hooks_file() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_2_copilot_hooks_file",
    );
}

#[test]
fn ac_0_8_2_cursor_hooks_json_version_1() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_2_cursor_hooks_json_version_1",
    );
}

#[test]
fn ac_0_8_2_opencode_plugin_copied() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_2_opencode_plugin_copied",
    );
}

#[test]
fn ac_0_8_3_install_all_twice_no_duplicates() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_3_install_all_twice_no_duplicates",
    );
}

#[test]
fn ac_0_8_3_uninstall_all_keeps_foreign_hooks() {
    py_case(
        "tools/test_install_hooks.py",
        "test_ac_0_8_3_uninstall_all_keeps_foreign_hooks",
    );
}

#[test]
fn ac_0_8_4_gemini_event_names_normalised() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_4_gemini_event_names_normalised",
    );
}

#[test]
fn ac_0_8_4_copilot_camelcase_payload_normalised() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_4_copilot_camelcase_payload_normalised",
    );
}

#[test]
fn ac_0_8_4_cursor_stop_status_normalised() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_4_cursor_stop_status_normalised",
    );
}

#[test]
fn ac_0_8_4_every_listed_agent_has_known_events() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_4_every_listed_agent_has_known_events",
    );
}

#[test]
fn ac_0_8_8_compact_shows_thinking_then_work() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_8_compact_shows_thinking_then_work",
    );
}

#[test]
fn ac_0_8_8_prompt_is_thinking_tools_are_working() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_8_prompt_is_thinking_tools_are_working",
    );
}

#[test]
fn ac_0_8_8_subagent_stop_does_not_idle_a_busy_agent() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_8_subagent_stop_does_not_idle_a_busy_agent",
    );
}

#[test]
fn ac_0_8_8_only_the_user_blocks_with_waiting() {
    py_case(
        "desktop/cross-platform/hooks/scripts/test_hooks.py",
        "test_ac_0_8_8_only_the_user_blocks_with_waiting",
    );
}
