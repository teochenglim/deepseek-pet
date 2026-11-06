//! AC0.3.x gates: event mapping, Z.ai registration, installer. Temp files only; the real
//! ~/.claude and ~/.zcode are never touched.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn state_map() -> serde_json::Value {
    let s = fs::read_to_string(root().join("desktop/cross-platform/config.example.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    v
}
fn state_of(ev: &str) -> String {
    state_map()["state_map"][ev]["state"]
        .as_str()
        .unwrap_or_else(|| panic!("state_map has no {ev}"))
        .to_string()
}
fn sandbox(label: &str) -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = std::env::temp_dir().join(format!("deepseek-pet-{label}-{n}"));
    fs::create_dir_all(d.join("zcode/cli")).unwrap();
    d
}
fn install(d: &Path, action: &str) {
    let hook = d.join("pet-hook.sh");
    fs::write(&hook, "#!/bin/sh\n").unwrap();
    let st = Command::new("python3")
        .arg("-I")
        .arg(root().join("tools/install_hooks.py"))
        .arg(action)
        .args(["--hook-script", hook.to_str().unwrap()])
        .args([
            "--claude-settings",
            d.join("claude/settings.json").to_str().unwrap(),
        ])
        .args([
            "--zcode-config",
            d.join("zcode/cli/config.json").to_str().unwrap(),
        ])
        .status()
        .expect("python3 missing");
    assert!(st.success());
}

#[test]
fn ac_0_3_1_event_map_running() {
    assert_eq!(state_of("PreToolUse"), "running");
    assert_eq!(state_of("UserPromptSubmit"), "running");
}
#[test]
fn ac_0_3_1_event_map_failed() {
    assert_eq!(state_of("StopFailure"), "failed");
    // failed must stay visible, so StopFailure is not a session-ending event
    let terminal = state_map()["terminal_events"].clone();
    assert!(!terminal
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e == "StopFailure"));
}
#[test]
fn ac_0_3_2_zai_events_all_mapped() {
    for ev in [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PermissionRequest",
        "PostToolUseFailure",
        "Stop",
    ] {
        state_of(ev);
    }
    assert_eq!(state_of("PostToolUseFailure"), "failed");
}
#[test]
fn ac_0_3_2_zai_installer_registers_events() {
    let d = sandbox("zai");
    install(&d, "install");
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(d.join("zcode/cli/config.json")).unwrap())
            .unwrap();
    assert_eq!(v["hooks"]["enabled"], true);
    for ev in [
        "PreToolUse",
        "PostToolUseFailure",
        "Stop",
        "PermissionRequest",
    ] {
        let cmd = v["hooks"]["events"][ev][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(cmd.contains("pet-hook.sh zai"), "{ev}: {cmd}");
    }
    let _ = fs::remove_dir_all(d);
}
#[test]
fn ac_0_3_4_install_idempotent() {
    let d = sandbox("idem");
    install(&d, "install");
    let a = fs::read_to_string(d.join("claude/settings.json")).unwrap();
    install(&d, "install");
    let b = fs::read_to_string(d.join("claude/settings.json")).unwrap();
    assert_eq!(a, b);
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    assert_eq!(v["hooks"]["Stop"].as_array().unwrap().len(), 1);
    let _ = fs::remove_dir_all(d);
}
#[test]
fn ac_0_3_4_install_valid_json() {
    let d = sandbox("valid");
    install(&d, "install");
    for f in ["claude/settings.json", "zcode/cli/config.json"] {
        let s = fs::read_to_string(d.join(f)).unwrap();
        serde_json::from_str::<serde_json::Value>(&s).unwrap();
    }
    install(&d, "uninstall");
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(d.join("claude/settings.json")).unwrap()).unwrap();
    assert!(
        v.get("hooks").is_none(),
        "uninstall must remove all managed hooks"
    );
    let _ = fs::remove_dir_all(d);
}
