//! AC0.7.x gates: hook events mapped to animations. The hook-script behaviour is tested by the
//! Python cases in hooks/scripts/test_hooks.py (MappingTests); each is run here by name.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/cross-platform")
}

fn py_case(name: &str) {
    let out = Command::new("python3")
        .current_dir(app())
        .args(["-m", "unittest", "hooks/scripts/test_hooks.py", "-k", name])
        .output()
        .expect("python3 is required to run the hook script tests");
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
fn ac_0_7_2_hook_uses_mapping_state() {
    py_case("test_ac_0_7_2_hook_uses_mapping_state");
}
#[test]
fn ac_0_7_2_unmapped_event_uses_default() {
    py_case("test_ac_0_7_2_unmapped_event_uses_default");
}
#[test]
fn ac_0_7_2_hook_reads_mapping_file_format() {
    py_case("test_ac_0_7_2_hook_reads_rust_written_mapping_file");
}
#[test]
fn ac_0_7_3_clip_mapping_emits_clip() {
    py_case("test_ac_0_7_3_clip_mapping_emits_clip");
}
#[test]
fn ac_0_7_3_clip_mapping_keeps_working_state() {
    py_case("test_ac_0_7_3_clip_mapping_keeps_working_state");
}
#[test]
fn ac_0_7_3_clip_reaches_the_payload() {
    py_case("test_ac_0_7_3_process_event_writes_clip_into_payload");
}

#[test]
fn ac_0_7_4_map_menu_lists_events_per_agent() {
    let src = app().join("src-tauri/src");
    let menus = fs::read_to_string(src.join("menus.rs")).unwrap();
    for needle in [
        "Map animations",
        "Reset all to defaults",
        "No animation",
        "Use last played",
        "map_menu(",
    ] {
        assert!(menus.contains(needle), "menus.rs lacks {needle}");
    }
    let mapping = fs::read_to_string(src.join("mapping.rs")).unwrap();
    for event in [
        "PreToolUse",
        "StopFailure",
        "PostToolUseFailure",
        "PermissionRequest",
    ] {
        assert!(mapping.contains(event), "agent table lacks {event}");
    }
    let lib = fs::read_to_string(src.join("lib.rs")).unwrap();
    assert!(
        lib.contains("menus::build_tray_menu"),
        "tray is not built from the mapping"
    );
}
