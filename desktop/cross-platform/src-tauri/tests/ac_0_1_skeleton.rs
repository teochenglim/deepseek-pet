//! AC0.1.x configuration gates. Visual/behavioural checks are manual (see RELEASE/v0.1.0.md).
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn conf() -> serde_json::Value {
    let s =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json")).unwrap();
    serde_json::from_str(&s).unwrap()
}
fn window() -> serde_json::Value {
    conf()["app"]["windows"][0].clone()
}
fn idle_frame() -> PathBuf {
    root().join("assets/big-blue-fish/frames/idle/00.png")
}

#[test]
fn ac_0_1_1_manifest_names_deepseek_pet() {
    let m = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    assert!(m.contains("name = \"deepseek-pet\""));
    assert_eq!(conf()["productName"], "DeepseekPet");
}
#[test]
fn ac_0_1_1_bundle_targets_include_app() {
    // absent `targets` means "all"; otherwise it must list app or dmg
    let t = &conf()["bundle"]["targets"];
    let ok = t.is_null()
        || t == "all"
        || t.as_array()
            .is_some_and(|a| a.iter().any(|v| v == "app" || v == "dmg"));
    assert!(ok, "bundle.targets must include app/dmg, got {t}");
}
#[test]
fn ac_0_1_1_bundle_icons_present() {
    // the installed app and dmg must carry the Big Blue Fish icon, not Tauri's default
    let icons = conf()["bundle"]["icon"].clone();
    let list = icons.as_array().expect("bundle.icon must list icons");
    assert!(list
        .iter()
        .any(|v| v.as_str().is_some_and(|s| s.ends_with("icon.icns"))));
    for v in list {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(v.as_str().unwrap());
        assert!(p.is_file(), "missing icon {p:?}");
    }
    assert!(root().join("assets/branding/icon-1024.png").is_file());
}
#[test]
fn ac_0_1_2_window_transparent() {
    assert_eq!(window()["transparent"], true);
}
#[test]
fn ac_0_1_2_window_undecorated() {
    assert_eq!(window()["decorations"], false);
    assert_eq!(window()["shadow"], false);
}
#[test]
fn ac_0_1_3_window_always_on_top() {
    assert_eq!(window()["alwaysOnTop"], true);
}
#[test]
fn ac_0_1_3_window_skips_taskbar() {
    assert_eq!(window()["skipTaskbar"], true);
}
#[test]
fn ac_0_1_4_idle_frame_is_png() {
    let b = fs::read(idle_frame()).expect("idle/00.png missing: add the Big Blue Fish still");
    assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n");
}
#[test]
fn ac_0_1_4_idle_frame_has_provenance() {
    let p = fs::read_to_string(root().join("assets/big-blue-fish/frames/PROVENANCE.md"))
        .expect("PROVENANCE.md missing");
    assert!(p.contains("idle/00.png") && p.contains(".webm"));
}
#[test]
fn ac_0_1_5_stop_hook_example_valid() {
    let s = fs::read_to_string(root().join("docs/hooks/claude-settings.example.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let cmd = v["hooks"]["Stop"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert!(cmd.ends_with("pet-hook.sh claude-code"));
}
#[test]
fn ac_0_1_5_hook_script_present() {
    assert!(root()
        .join("desktop/cross-platform/hooks/pet-hook.sh")
        .is_file());
}
