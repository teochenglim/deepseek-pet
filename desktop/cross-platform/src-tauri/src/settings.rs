//! App settings that the user changes from the menu-bar icon, stored in `~/.deepseek-pet/settings.json`:
//!
//! ```json
//! {"version": 1, "active_source": "claude-code"}
//! ```
//! `active_source` absent means every source drives the pet (the default).
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_source: Option<String>,
}

impl Settings {
    /// A missing or invalid file gives the defaults, never an error.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut value = serde_json::to_value(self).map_err(io::Error::other)?;
        value["version"] = serde_json::Value::from(1);
        let text = serde_json::to_string_pretty(&value).map_err(io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text + "\n")?;
        std::fs::rename(tmp, path)
    }
}

pub struct SettingsStore(pub Mutex<Settings>);

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> std::path::PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = std::env::temp_dir().join(format!("deepseek-pet-settings-{label}-{n}"));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn ac_0_8_7_active_source_survives_a_restart() {
        let dir = temp("roundtrip");
        let path = dir.join("settings.json");
        let s = Settings {
            active_source: Some("claude-code".into()),
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        Settings::default().save(&path).unwrap();
        assert_eq!(Settings::load(&path).active_source, None, "all sources");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ac_0_8_7_bad_settings_file_means_all_sources() {
        let dir = temp("bad");
        let path = dir.join("settings.json");
        assert_eq!(Settings::load(&path), Settings::default());
        std::fs::write(&path, "{nope").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
