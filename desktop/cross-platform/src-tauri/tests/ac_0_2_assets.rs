//! AC0.2.x gates over the converted Big Blue Fish assets and NOTICE.
use std::fs;
use std::path::{Path, PathBuf};

const STATES: [&str; 3] = ["idle", "running-right", "failed"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn frames() -> PathBuf {
    root().join("assets/big-blue-fish/frames")
}
fn pet() -> serde_json::Value {
    let s = fs::read_to_string(root().join("assets/big-blue-fish/pet.json")).unwrap();
    serde_json::from_str(&s).unwrap()
}
fn png_size(p: &Path) -> (u32, u32) {
    let b = fs::read(p).unwrap();
    assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n", "{p:?} is not a PNG");
    let w = u32::from_be_bytes(b[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(b[20..24].try_into().unwrap());
    (w, h)
}

#[test]
fn ac_0_2_1_convert_three_states() {
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(frames().join("frames-manifest.json")).unwrap())
            .unwrap();
    for s in STATES {
        let row = manifest["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["state"] == s)
            .unwrap_or_else(|| panic!("manifest has no row for {s}"));
        assert_eq!(row["method"], "ffmpeg", "{s} not produced by the converter");
        let n = row["frames"].as_array().unwrap().len();
        assert!((1..=8).contains(&n), "{s} has {n} frames");
        for f in row["frames"].as_array().unwrap() {
            assert!(frames().join(s).join(f.as_str().unwrap()).is_file());
        }
    }
}
#[test]
fn ac_0_2_1_cell_size_192x208() {
    for s in STATES {
        for e in fs::read_dir(frames().join(s)).unwrap() {
            let p = e.unwrap().path();
            assert_eq!(png_size(&p), (192, 208), "{p:?}");
        }
    }
}
#[test]
fn ac_0_2_2_notice_has_asset_license() {
    let n = fs::read_to_string(root().join("NOTICE")).unwrap();
    assert!(n.contains("big-blue-fish-desktop-pet") && n.contains("Apache License 2.0"));
}
#[test]
fn ac_0_2_2_notice_has_code_license() {
    let n = fs::read_to_string(root().join("NOTICE")).unwrap();
    assert!(n.contains("golitter/coding-agent-pet") && n.contains("MIT"));
}
#[test]
fn ac_0_2_3_pet_json_records_source() {
    let p = pet();
    for s in STATES {
        let m = &p["mapped"][s];
        assert!(m["source"].as_str().unwrap().ends_with(".webm"), "{s}");
        assert_eq!(m["original_resolution"].as_array().unwrap().len(), 2, "{s}");
    }
}
#[test]
fn ac_0_2_3_pet_json_frame_counts_match_disk() {
    let p = pet();
    for s in STATES {
        let on_disk = fs::read_dir(frames().join(s)).unwrap().count() as u64;
        assert_eq!(p["mapped"][s]["frames"].as_u64().unwrap(), on_disk, "{s}");
    }
    assert!(!p["unmapped"].as_array().unwrap().is_empty());
}
