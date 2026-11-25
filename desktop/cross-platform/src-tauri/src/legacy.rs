//! v0.9.0 to v0.9.4 could install a Claude Code status line that ran this app's binary with
//! `--statusline`. That feature is gone, but copies of it may still be configured, so:
//! - the app still answers `--statusline` (prints nothing, exits at once) instead of opening a
//!   second pet every time Claude Code refreshes its status line;
//! - on launch it removes its own leftover `statusLine` entry from `~/.claude/settings.json` and
//!   the `~/.deepseek-pet/bin/statusline` link. A status line of the user's own is never touched.
use serde_json::Value;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const MARKER: &str = "# deepseek-pet";

/// `deepseek-pet --statusline` from an old settings entry or an already open Claude Code session.
pub fn run() -> i32 {
    let _ = io::stdin().read_to_end(&mut Vec::new());
    0
}

pub fn claude_settings_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude/settings.json")
}

fn is_ours(line: &Value) -> bool {
    line.get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.trim_end().ends_with(MARKER))
}

/// Remove our old status line setting and link. Returns whether anything was removed.
/// A settings file that is missing or not a JSON object is left alone.
pub fn cleanup(settings: &Path, home: &Path) -> io::Result<bool> {
    let mut removed = false;
    let link = home.join("bin").join("statusline");
    if std::fs::symlink_metadata(&link).is_ok_and(|m| m.file_type().is_symlink()) {
        std::fs::remove_file(&link)?;
        removed = true;
    }
    let Ok(text) = std::fs::read_to_string(settings) else {
        return Ok(removed);
    };
    let Some(mut doc) = serde_json::from_str::<Value>(&text)
        .ok()
        .filter(Value::is_object)
    else {
        return Ok(removed);
    };
    if !doc.get("statusLine").is_some_and(is_ours) {
        return Ok(removed);
    }
    let bak = settings.with_file_name("settings.json.deepseek-pet.bak");
    if !bak.exists() {
        std::fs::copy(settings, &bak)?;
    }
    if let Some(map) = doc.as_object_mut() {
        map.remove("statusLine");
    }
    let out = serde_json::to_string_pretty(&doc).map_err(io::Error::other)?;
    crate::home::write_atomic(settings, &(out + "\n"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("deepseek-pet-legacy-{label}-{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ac_0_9_10_leftover_statusline_is_removed_on_launch() {
        let root = temp("ours");
        let settings = root.join("settings.json");
        let home = root.join("pet");
        std::fs::create_dir_all(home.join("bin")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/Applications/Pet", home.join("bin/statusline")).unwrap();
        std::fs::write(
            &settings,
            r#"{"zeta": 1, "statusLine": {"type": "command", "command": "'/x/bin/statusline' --statusline # deepseek-pet"}, "alpha": 2}"#,
        )
        .unwrap();
        assert!(cleanup(&settings, &home).unwrap());
        let text = std::fs::read_to_string(&settings).unwrap();
        assert!(!text.contains("statusLine"), "{text}");
        assert!(
            text.find("zeta").unwrap() < text.find("alpha").unwrap(),
            "key order kept"
        );
        assert!(root.join("settings.json.deepseek-pet.bak").is_file());
        assert!(std::fs::symlink_metadata(home.join("bin/statusline")).is_err());
        assert!(!cleanup(&settings, &home).unwrap(), "second run is a no-op");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ac_0_9_10_the_users_own_statusline_and_bad_files_are_left_alone() {
        let root = temp("foreign");
        let settings = root.join("settings.json");
        let mine = r#"{"statusLine":{"type":"command","command":"my-line.sh"}}"#;
        std::fs::write(&settings, mine).unwrap();
        assert!(!cleanup(&settings, &root).unwrap());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), mine);
        std::fs::write(&settings, "{ not json").unwrap();
        assert!(!cleanup(&settings, &root).unwrap());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), "{ not json");
        assert!(!cleanup(&root.join("missing.json"), &root).unwrap());
        let _ = std::fs::remove_dir_all(root);
    }
}
