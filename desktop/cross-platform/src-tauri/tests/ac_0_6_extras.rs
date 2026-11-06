//! AC0.6.x gates: every Big Blue Fish clip can be triggered by hand from the menu-bar icon.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn extras() -> PathBuf {
    root().join("assets/big-blue-fish/frames/extras")
}
fn clips() -> Vec<serde_json::Value> {
    let s = fs::read_to_string(extras().join("catalog.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    v["clips"].as_array().unwrap().clone()
}
fn png_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    v.sort();
    v
}
fn dir_size(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() {
                dir_size(&p)
            } else {
                fs::metadata(&p).unwrap().len()
            }
        })
        .sum()
}
fn node_case(name: &str) {
    let out = Command::new("node")
        .current_dir(root().join("desktop/cross-platform"))
        .args(["--test", "--test-reporter=tap"])
        .arg(format!("--test-name-pattern=^{name}$"))
        .arg("hooks/tests/animator-extras.test.mjs")
        .output()
        .expect("node is required to run the extras tests");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{name} failed:\n{s}");
    assert!(s.contains("# pass 1"), "{name} matched no test:\n{s}");
}

#[test]
fn ac_0_6_1_catalog_lists_all_clips() {
    let c = clips();
    assert_eq!(c.len(), 106, "every Big Blue Fish clip must be playable");
    let mut ids: Vec<&str> = c.iter().map(|x| x["id"].as_str().unwrap()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 106, "clip ids must be unique");
    for clip in &c {
        for key in ["title", "title_zh", "category", "source"] {
            assert!(
                !clip[key].as_str().unwrap_or("").is_empty(),
                "{clip} lacks {key}"
            );
        }
    }
}
#[test]
fn ac_0_6_1_extra_frames_on_disk() {
    for clip in clips() {
        let id = clip["id"].as_str().unwrap();
        let n = clip["frames"].as_u64().unwrap() as usize;
        assert_eq!(png_files(&extras().join(id)).len(), n, "{id}");
    }
}
#[test]
fn ac_0_6_2_extras_stay_out_of_startup_manifest() {
    let m = fs::read_to_string(root().join("assets/big-blue-fish/frames/frames-manifest.json"))
        .unwrap();
    assert!(
        !m.contains("extra"),
        "extras must load on demand, not at startup"
    );
}
#[test]
fn ac_0_6_2_extras_under_size_budget() {
    let mb = dir_size(&extras()) / 1_000_000;
    assert!(
        mb <= 30,
        "extras are {mb} MB, budget is 30 MB (bundle target 60 MB)"
    );
}
#[test]
fn ac_0_6_3_extra_loads_lazily_and_plays_once() {
    node_case("ac_0_6_3_extra_loads_lazily_and_plays_once");
}
#[test]
fn ac_0_6_3_extra_frames_freed_after_play() {
    node_case("ac_0_6_3_extra_frames_freed_after_play");
}
#[test]
fn ac_0_6_3_state_change_during_extra_is_kept() {
    node_case("ac_0_6_3_state_change_during_extra_is_kept");
}
#[test]
fn ac_0_6_4_tray_has_play_menu() {
    let base = root().join("desktop/cross-platform");
    let lib = fs::read_to_string(base.join("src-tauri/src/lib.rs")).unwrap();
    let menus = fs::read_to_string(base.join("src-tauri/src/menus.rs")).unwrap();
    assert!(lib.contains("menus::build_tray_menu"));
    assert!(menus.contains("\"Play animation\"") && menus.contains("play_entries(app"));
    assert!(menus.contains("play:random") && menus.contains("\"play-animation\""));
    let main = fs::read_to_string(base.join("src/main.js")).unwrap();
    assert!(main.contains("\"play-animation\"") && main.contains("playExtra"));
}
#[test]
fn ac_0_6_4_extra_frames_are_cell_sized() {
    for clip in clips() {
        let id = clip["id"].as_str().unwrap();
        let first = &png_files(&extras().join(id))[0];
        let b = fs::read(first).unwrap();
        assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n");
        let w = u32::from_be_bytes(b[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(b[20..24].try_into().unwrap());
        assert_eq!((w, h), (192, 208), "{id}");
    }
}

#[test]
fn ac_0_6_5_right_click_opens_native_menu_directly() {
    let base = root().join("desktop/cross-platform");
    let ctl = fs::read_to_string(base.join("src/interaction-controller.js")).unwrap();
    assert!(
        ctl.contains("invoke(\"show_pet_menu\")"),
        "right-click must open the native menu"
    );
    let lib = fs::read_to_string(base.join("src-tauri/src/lib.rs")).unwrap();
    assert!(lib.contains("commands::show_pet_menu") && lib.contains("menus::POPUP_PREFIX"));
    let menus = fs::read_to_string(base.join("src-tauri/src/menus.rs")).unwrap();
    for needle in [
        "Random animation",
        "Hide for now",
        "\"Quit\"",
        "play_entries",
    ] {
        assert!(menus.contains(needle), "pet_popup lacks {needle}");
    }
    // no extra "Play animation" click in the in-window menu any more
    let js = fs::read_to_string(base.join("src/context-menu.js")).unwrap();
    assert!(!js.contains("show_play_menu"));
}
