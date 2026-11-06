//! AC0.4.x gates. Animation behaviour runs the real SpriteAnimator under Node
//! (hooks/tests/animator-interaction.test.mjs); the rest checks config and wiring.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn app() -> PathBuf {
    root().join("desktop/cross-platform")
}

/// Run one named case from the Node test file and require that exactly one test passed.
fn node_case(name: &str) {
    let out = Command::new("node")
        .current_dir(app())
        .args(["--test", "--test-reporter=tap"])
        .arg(format!("--test-name-pattern=^{name}$"))
        .arg("hooks/tests/animator-interaction.test.mjs")
        .output()
        .expect("node is required to run the interaction tests");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{name} failed:\n{s}");
    assert!(s.contains("# pass 1"), "{name} matched no test:\n{s}");
}

#[test]
fn ac_0_4_1_drag_returns_state() {
    node_case("ac_0_4_1_drag_returns_state");
}
#[test]
fn ac_0_4_1_drag_direction_picks_row() {
    node_case("ac_0_4_1_drag_direction_picks_row");
}
#[test]
fn ac_0_4_2_click_wave_returns_to_running() {
    node_case("ac_0_4_2_click_wave_returns_to_running");
}
#[test]
fn ac_0_4_2_hover_jump_only_when_idle() {
    node_case("ac_0_4_2_hover_jump_only_when_idle");
}
#[test]
fn ac_0_4_2_click_wave_follows_state_change_mid_wave() {
    node_case("ac_0_4_2_click_wave_follows_state_change_mid_wave");
}

#[test]
fn ac_0_4_4_menu_has_quit_and_hide() {
    let s = fs::read_to_string(app().join("config.example.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let actions: Vec<&str> = v["menu"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["action"].as_str())
        .collect();
    assert!(
        actions.contains(&"quit") && actions.contains(&"hide"),
        "{actions:?}"
    );
}
#[test]
fn ac_0_4_4_hide_restorable_via_tray() {
    let lib = fs::read_to_string(app().join("src-tauri/src/lib.rs")).unwrap();
    assert!(lib.contains("TrayIconBuilder"), "no menu-bar icon");
    let menus = fs::read_to_string(app().join("src-tauri/src/menus.rs")).unwrap();
    assert!(
        menus.contains("Action::Show => crate::commands::show_pet"),
        "tray cannot restore the pet"
    );
    assert!(
        lib.contains("menus::parse_action"),
        "tray not wired to the shared actions"
    );
    assert!(
        lib.contains("commands::hide_pet"),
        "hide_pet command not registered"
    );
    let menu = fs::read_to_string(app().join("src/context-menu.js")).unwrap();
    assert!(
        menu.contains("\"hide_pet\""),
        "context menu does not call hide_pet"
    );
}
#[test]
fn ac_0_4_5_all_states_converted() {
    let m = fs::read_to_string(root().join("assets/big-blue-fish/frames/frames-manifest.json"))
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&m).unwrap();
    for s in [
        "idle",
        "running",
        "running-left",
        "running-right",
        "failed",
        "waiting",
        "review",
        "waving",
        "jumping",
    ] {
        let row = v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["state"] == s)
            .unwrap_or_else(|| panic!("no row for {s}"));
        assert_eq!(row["method"], "ffmpeg", "{s} is still a placeholder");
    }
}
#[test]
fn ac_0_4_5_right_run_is_mirrored_left_run() {
    let p = fs::read_to_string(root().join("assets/big-blue-fish/pet.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&p).unwrap();
    let (l, r) = (&v["mapped"]["running-left"], &v["mapped"]["running-right"]);
    assert_eq!(l["source"], r["source"]);
    assert_eq!(r["flip"], true);
    assert_eq!(l["flip"], false);
}
