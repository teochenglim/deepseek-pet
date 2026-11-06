//! User choices for "which animation does this hook event play", stored in
//! `~/.deepseek-pet/mapping.json`:
//!
//! ```json
//! {"version": 1, "agents": {"claude-code": {"Stop": {"clip": "c042"}, "PreToolUse": {"state": "review"}}}}
//! ```
//! The hook scripts read the same file on every event, so changes apply without a restart.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::Mutex;

/// Agents that can be mapped, with the events each one sends (kept in line with
/// `tools/install_hooks.py`; a test checks that).
pub const AGENTS: &[(&str, &str, &[&str])] = &[
    (
        "claude-code",
        "Claude Code",
        &[
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "PermissionRequest",
            "Notification",
            "PreCompact",
            "SubagentStop",
            "Stop",
            "StopFailure",
            "SessionEnd",
        ],
    ),
    (
        "zai",
        "Z.ai ZCode",
        &[
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "PostToolUseFailure",
            "Stop",
        ],
    ),
];

/// States a hook event can switch the pet to (the drag-only run states are left out).
pub const MAPPABLE_STATES: &[(&str, &str)] = &[
    ("idle", "Idle"),
    ("running", "Running"),
    ("waiting", "Waiting"),
    ("review", "Thinking"),
    ("failed", "Failed"),
    ("waving", "Waving"),
    ("jumping", "Jumping"),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Choice {
    /// Show this state instead of the default one.
    State(String),
    /// Play this clip once when the event arrives.
    Clip(String),
    /// No animation for this event (the pet goes idle).
    Off(bool),
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    #[serde(default)]
    pub agents: BTreeMap<String, BTreeMap<String, Choice>>,
}

impl Mapping {
    /// A missing, empty or invalid file gives the defaults, never an error.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Mapping>(&text).ok())
            .unwrap_or_default()
    }

    pub fn get(&self, agent: &str, event: &str) -> Option<&Choice> {
        self.agents.get(agent).and_then(|events| events.get(event))
    }

    /// `None` removes the entry (back to the default).
    pub fn set(&mut self, agent: &str, event: &str, choice: Option<Choice>) {
        match choice {
            Some(c) => {
                self.agents
                    .entry(agent.to_string())
                    .or_default()
                    .insert(event.to_string(), c);
            }
            None => {
                if let Some(events) = self.agents.get_mut(agent) {
                    events.remove(event);
                    if events.is_empty() {
                        self.agents.remove(agent);
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.agents.clear();
    }

    /// Atomic write, with a one-time backup of what was there.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if path.exists() {
            let bak = path.with_extension("json.bak");
            if !bak.exists() {
                std::fs::copy(path, bak)?;
            }
        }
        #[derive(Serialize)]
        struct File<'a> {
            version: u32,
            agents: &'a BTreeMap<String, BTreeMap<String, Choice>>,
        }
        let text = serde_json::to_string_pretty(&File {
            version: 1,
            agents: &self.agents,
        })
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text + "\n")?;
        std::fs::rename(tmp, path)
    }
}

/// Mapping shared by the menus; `last_played` feeds "Assign last played clip".
pub struct MappingStore {
    pub mapping: Mutex<Mapping>,
    pub last_played: Mutex<Option<String>>,
}

impl MappingStore {
    pub fn new(mapping: Mapping) -> Self {
        Self {
            mapping: Mutex::new(mapping),
            last_played: Mutex::new(None),
        }
    }
}

/// "Assign last played clip" is only offered once a clip has been played.
pub fn last_item_enabled(last_played: Option<&str>) -> bool {
    last_played.is_some()
}

/// The choice "Assign last played clip" stores, if there is one.
pub fn assign_last(last_played: Option<&str>) -> Option<Choice> {
    last_played.map(|id| Choice::Clip(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> std::path::PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("deepseek-pet-map-{label}-{n}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn ac_0_7_1_mapping_roundtrip_and_override() {
        let dir = temp("roundtrip");
        let path = dir.join("mapping.json");
        let mut m = Mapping::default();
        m.set("claude-code", "Stop", Some(Choice::Clip("c042".into())));
        m.set(
            "claude-code",
            "PreToolUse",
            Some(Choice::State("review".into())),
        );
        m.set("zai", "PostToolUseFailure", Some(Choice::Off(true)));
        m.save(&path).unwrap();
        let back = Mapping::load(&path);
        assert_eq!(back, m);
        assert_eq!(
            back.get("claude-code", "Stop"),
            Some(&Choice::Clip("c042".into()))
        );
        assert_eq!(
            back.get("claude-code", "SessionEnd"),
            None,
            "unmapped stays default"
        );
        // the file is what the hook scripts read
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"clip\": \"c042\"") && text.contains("\"version\": 1"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ac_0_7_2_python_reads_rust_written_mapping() {
        // the file Rust writes must be what the hook scripts parse
        let dir = temp("py");
        let path = dir.join("mapping.json");
        let mut m = Mapping::default();
        m.set("zai", "Stop", Some(Choice::Clip("c042".into())));
        m.save(&path).unwrap();
        let scripts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hooks/scripts");
        let code = format!(
            "import sys, json; sys.path.insert(0, {:?}); import common; \
             print(json.dumps(common.load_mapping({:?})))",
            scripts.to_string_lossy(),
            path.to_string_lossy()
        );
        let out = std::process::Command::new("python3")
            .args(["-c", &code])
            .output()
            .expect("python3 is required");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["zai"]["Stop"]["clip"], "c042");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ac_0_7_1_bad_mapping_file_falls_back() {
        let dir = temp("bad");
        let path = dir.join("mapping.json");
        assert_eq!(Mapping::load(&path), Mapping::default(), "missing file");
        std::fs::write(&path, "").unwrap();
        assert_eq!(Mapping::load(&path), Mapping::default(), "empty file");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(Mapping::load(&path), Mapping::default(), "invalid json");
        std::fs::write(&path, r#"{"agents":{"zai":{"Stop":{"bogus":1}}}}"#).unwrap();
        assert_eq!(
            Mapping::load(&path),
            Mapping::default(),
            "unknown choice kind"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ac_0_7_1_save_keeps_a_backup_once() {
        let dir = temp("bak");
        let path = dir.join("mapping.json");
        std::fs::write(&path, r#"{"agents":{}}"#).unwrap();
        let mut m = Mapping::default();
        m.set("zai", "Stop", Some(Choice::State("idle".into())));
        m.save(&path).unwrap();
        m.set("zai", "Stop", Some(Choice::State("failed".into())));
        m.save(&path).unwrap();
        let bak = std::fs::read_to_string(dir.join("mapping.json.bak")).unwrap();
        assert_eq!(bak, r#"{"agents":{}}"#, "the first original is kept");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ac_0_7_4_reset_restores_defaults() {
        let mut m = Mapping::default();
        m.set("claude-code", "Stop", Some(Choice::Off(true)));
        m.set("zai", "Stop", Some(Choice::State("waving".into())));
        m.reset();
        assert_eq!(m, Mapping::default());
        // clearing the last event of an agent drops the agent too
        m.set("zai", "Stop", Some(Choice::Off(true)));
        m.set("zai", "Stop", None);
        assert!(m.agents.is_empty());
    }

    #[test]
    fn ac_0_7_5_assign_last_played_clip() {
        assert_eq!(assign_last(Some("c042")), Some(Choice::Clip("c042".into())));
        assert!(last_item_enabled(Some("c042")));
    }

    #[test]
    fn ac_0_7_5_no_last_played_disables_item() {
        assert!(!last_item_enabled(None));
        assert_eq!(assign_last(None), None);
    }
}
