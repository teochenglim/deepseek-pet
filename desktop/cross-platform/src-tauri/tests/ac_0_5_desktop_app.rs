//! AC0.5.x gates: the pet behaves like a normal macOS app (Dock, menu bar) and floats on top.
use std::fs;
use std::path::{Path, PathBuf};

fn app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../desktop/cross-platform")
}
fn lib_rs() -> String {
    fs::read_to_string(app().join("src-tauri/src/lib.rs")).unwrap()
}
fn window() -> serde_json::Value {
    let s = fs::read_to_string(app().join("src-tauri/tauri.conf.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    v["app"]["windows"][0].clone()
}

#[test]
fn ac_0_5_1_dock_visible_and_reopen() {
    let lib = lib_rs();
    assert!(
        lib.contains("ActivationPolicy::Regular"),
        "app must show in the Dock"
    );
    assert!(!lib.contains("ActivationPolicy::Accessory"));
    assert!(
        lib.contains("RunEvent::Reopen"),
        "Dock click must restore a hidden pet"
    );
}
#[test]
fn ac_0_5_1_tray_icon_present() {
    let png = fs::read(app().join("src-tauri/icons/tray.png")).expect("tray.png missing");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let lib = lib_rs();
    assert!(lib.contains("icons/tray.png") && lib.contains("icon_as_template(false)"));
}
#[test]
fn ac_0_5_2_floats_on_top_on_every_space() {
    let w = window();
    assert_eq!(w["alwaysOnTop"], true);
    assert_eq!(w["visibleOnAllWorkspaces"], true);
}
#[test]
fn ac_0_5_2_nothing_lowers_the_window() {
    // a Regular app must not drop the floating level when it loses focus
    assert!(!lib_rs().contains("set_always_on_top(false)"));
    assert_eq!(window()["decorations"], false);
}
