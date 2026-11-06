//! AC0.8.11: PreToolUse defaults to working with a random pool, and each agent has its own clip throttle.
use std::fs;
use std::path::{Path, PathBuf};

fn platform() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn ac_0_8_11_pretooluse_defaults_to_working_with_a_random_pool() {
    let text = fs::read_to_string(platform().join("config.example.json")).unwrap();
    let cfg: serde_json::Value = serde_json::from_str(&text).unwrap();
    let entry = &cfg["state_map"]["PreToolUse"];
    assert_eq!(entry["state"], "running");
    assert!(entry["clips"].as_array().map_or(0, Vec::len) >= 3);
}

#[test]
fn ac_0_8_11_hook_clip_stamp_is_per_source() {
    let py = fs::read_to_string(platform().join("hooks/scripts/common.py")).unwrap();
    assert!(
        py.contains(".last-default-clip-{source}"),
        "a shared stamp lets one busy agent silence another"
    );
    assert!(!py.contains("'.last-default-clip')"));
}
