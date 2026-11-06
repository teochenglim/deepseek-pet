//! Where the app keeps its files when it runs as an installed `.app` instead of from the repository.
//!
//! Read-only parts (frames, default config, hook scripts) ship inside `Contents/Resources`.
//! Everything that changes at run time lives in `~/.deepseek-pet/` (override: `DEEPSEEK_PET_HOME`),
//! so the app can be replaced or moved without losing the user's config, sessions or hook scripts.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `~/.deepseek-pet`, or `$DEEPSEEK_PET_HOME` when set.
pub fn user_home_dir() -> PathBuf {
    if let Some(custom) = std::env::var_os("DEEPSEEK_PET_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(custom);
    }
    match std::env::var_os("HOME").filter(|v| !v.is_empty()) {
        Some(home) => PathBuf::from(home).join(".deepseek-pet"),
        None => PathBuf::from("/tmp/.deepseek-pet"),
    }
}

/// `Foo.app/Contents/Resources` when `exe` is `Foo.app/Contents/MacOS/<binary>` and that folder exists.
/// `None` for a development run from the repository.
pub fn bundle_resources(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    if macos.file_name()? != "MacOS" {
        return None;
    }
    let contents = macos.parent()?;
    if contents.file_name()? != "Contents" {
        return None;
    }
    let resources = contents.join("Resources");
    resources.is_dir().then_some(resources)
}

fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_name() == "__pycache__" {
            continue;
        }
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                make_executable(&path);
            } else if path.extension().is_some_and(|e| e == "sh" || e == "py") {
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
            }
        }
    }
}
#[cfg(not(unix))]
fn make_executable(_dir: &Path) {}

/// Prepare `home` for an installed app:
/// - `hooks/` is refreshed from the bundle on every launch (our scripts, safe to overwrite);
/// - `runtime/sessions/` exists;
/// - `config.json` is created from the bundled default once and then left alone (user edits survive updates).
pub fn ensure_user_home(home: &Path, resources: &Path) -> io::Result<()> {
    fs::create_dir_all(home.join("runtime").join("sessions"))?;

    let bundled_hooks = resources.join("hooks");
    if bundled_hooks.is_dir() {
        copy_dir(&bundled_hooks, &home.join("hooks"))?;
        make_executable(&home.join("hooks"));
    }

    let config_path = home.join("config.json");
    if !config_path.exists() {
        let default = fs::read_to_string(resources.join("config.example.json"))?;
        let mut value: serde_json::Value = serde_json::from_str(&default)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        // Hook scripts and the backend must watch the same folder, so make it explicit.
        value["sessions_dir"] = serde_json::Value::String(
            home.join("runtime")
                .join("sessions")
                .to_string_lossy()
                .to_string(),
        );
        let text = serde_json::to_string_pretty(&value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&config_path, text + "\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let d = std::env::temp_dir().join(format!("deepseek-pet-home-{label}-{n}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn fake_resources(root: &Path, hook_body: &str) -> PathBuf {
        let res = root.join("Foo.app/Contents/Resources");
        fs::create_dir_all(res.join("hooks/scripts")).unwrap();
        fs::write(res.join("hooks/pet-hook.sh"), hook_body).unwrap();
        fs::write(res.join("hooks/scripts/claude_hook.py"), "# hook\n").unwrap();
        fs::write(
            res.join("config.example.json"),
            r#"{"pet_id":"big-blue-fish","state_map":{"Stop":{"state":"jumping"}}}"#,
        )
        .unwrap();
        res
    }

    #[test]
    fn ac_0_6_7_resolves_bundled_frames() {
        let root = temp("bundle");
        let res = fake_resources(&root, "#!/bin/sh\n");
        let exe = root.join("Foo.app/Contents/MacOS/deepseek-pet");
        assert_eq!(bundle_resources(&exe), Some(res));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ac_0_6_7_dev_run_still_uses_repo() {
        let dev = Path::new("/repo/desktop/cross-platform/src-tauri/target/debug/deepseek-pet");
        assert_eq!(bundle_resources(dev), None);
        // a MacOS folder without Contents/Resources next to it is not a bundle either
        assert_eq!(bundle_resources(Path::new("/x/MacOS/bin")), None);
    }

    #[test]
    fn ac_0_6_7_user_home_can_be_overridden() {
        std::env::set_var("DEEPSEEK_PET_HOME", "/tmp/custom-pet-home");
        assert_eq!(user_home_dir(), PathBuf::from("/tmp/custom-pet-home"));
        std::env::remove_var("DEEPSEEK_PET_HOME");
        assert!(user_home_dir().ends_with(".deepseek-pet"));
    }

    #[test]
    fn ac_0_6_8_user_state_dir() {
        let root = temp("state");
        let res = fake_resources(&root, "#!/bin/sh\necho v1\n");
        let home = root.join("home");
        ensure_user_home(&home, &res).unwrap();
        assert!(home.join("runtime/sessions").is_dir());
        assert!(home.join("hooks/pet-hook.sh").is_file());
        assert!(home.join("hooks/scripts/claude_hook.py").is_file());
        let cfg: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(home.join("config.json")).unwrap()).unwrap();
        let sessions = cfg["sessions_dir"].as_str().unwrap();
        assert!(Path::new(sessions).is_absolute() && sessions.ends_with("runtime/sessions"));
        assert_eq!(cfg["state_map"]["Stop"]["state"], "jumping");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(home.join("hooks/pet-hook.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert!(mode & 0o111 != 0, "pet-hook.sh must be executable");
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ac_0_6_8_update_keeps_user_config() {
        let root = temp("update");
        let home = root.join("home");
        ensure_user_home(&home, &fake_resources(&root, "#!/bin/sh\necho v1\n")).unwrap();
        fs::write(home.join("config.json"), r#"{"scale": 0.9}"#).unwrap();
        // a new app version ships a changed hook script
        ensure_user_home(&home, &fake_resources(&root, "#!/bin/sh\necho v2\n")).unwrap();
        assert_eq!(
            fs::read_to_string(home.join("config.json")).unwrap(),
            r#"{"scale": 0.9}"#,
            "user config must survive an update"
        );
        assert!(fs::read_to_string(home.join("hooks/pet-hook.sh"))
            .unwrap()
            .contains("v2"));
        let _ = fs::remove_dir_all(root);
    }
}
