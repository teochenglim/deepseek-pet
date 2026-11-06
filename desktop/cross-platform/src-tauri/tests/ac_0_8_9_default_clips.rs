//! AC0.8.9: working events default to a random pool of Work clips, not the head-turning one.
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn json(rel: &str) -> serde_json::Value {
    let s = fs::read_to_string(root().join(rel)).unwrap();
    serde_json::from_str(&s).unwrap()
}
fn pool(ev: &str) -> Vec<String> {
    json("desktop/cross-platform/config.example.json")["state_map"][ev]["clips"]
        .as_array()
        .unwrap_or_else(|| panic!("{ev} has no clips pool"))
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn ac_0_8_9_working_events_have_a_varied_pool() {
    for ev in ["PreToolUse", "SubagentStop", "PostCompact"] {
        let p = pool(ev);
        assert!(p.len() >= 3, "{ev} needs several clips to be random");
        assert!(
            !p.contains(&"c052".to_string()),
            "{ev} must not use the head-turning pacing clip"
        );
    }
}

#[test]
fn ac_0_8_9_every_pool_clip_exists_in_the_catalog() {
    let cat = json("assets/big-blue-fish/frames/extras/catalog.json");
    let ids: Vec<&str> = cat["clips"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    for ev in [
        "UserPromptSubmit",
        "PreToolUse",
        "SubagentStop",
        "PreCompact",
        "PostCompact",
    ] {
        for c in pool(ev) {
            assert!(ids.contains(&c.as_str()), "{ev}: unknown clip {c}");
        }
    }
}

#[test]
fn ac_0_8_9_config_version_bumped_so_old_configs_upgrade() {
    let v = json("desktop/cross-platform/config.example.json")["config_version"]
        .as_u64()
        .unwrap();
    assert!(v >= 3);
}
