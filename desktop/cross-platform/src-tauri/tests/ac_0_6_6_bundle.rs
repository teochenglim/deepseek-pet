//! AC0.6.6: the installed .app carries everything it needs (frames, default config, hook scripts).
use std::fs;
use std::path::{Path, PathBuf};

fn tauri_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}
fn resources() -> serde_json::Map<String, serde_json::Value> {
    let s = fs::read_to_string(tauri_dir().join("tauri.conf.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    v["bundle"]["resources"]
        .as_object()
        .expect("bundle.resources must be a map of source to target")
        .clone()
}

#[test]
fn ac_0_6_6_bundle_resources_listed() {
    let r = resources();
    let targets: Vec<&str> = r.values().filter_map(|v| v.as_str()).collect();
    for want in [
        "assets/big-blue-fish/frames",
        "config.example.json",
        "hooks",
        "hooks/install_hooks.py",
    ] {
        assert!(
            targets.contains(&want),
            "bundle.resources lacks target {want}: {targets:?}"
        );
    }
}

#[test]
fn ac_0_6_6_resource_sources_exist() {
    for source in resources().keys() {
        let p = tauri_dir().join(source);
        assert!(
            p.exists(),
            "resource source {source} does not exist at {p:?}"
        );
    }
}

#[test]
fn ac_0_6_6_built_bundle_contains_frames() {
    let res = tauri_dir().join("target/release/bundle/macos/DeepseekPet.app/Contents/Resources");
    if !res.is_dir() {
        eprintln!("skipped: no built bundle (run `make build`)");
        return;
    }
    for rel in [
        "assets/big-blue-fish/frames/frames-manifest.json",
        "assets/big-blue-fish/frames/idle/00.png",
        "assets/big-blue-fish/frames/extras/catalog.json",
        "config.example.json",
        "hooks/pet-hook.sh",
        "hooks/scripts/common.py",
        "hooks/install_hooks.py",
    ] {
        assert!(res.join(rel).is_file(), "built bundle lacks {rel}");
    }
}
