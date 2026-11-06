//! Installing the pet's hooks into the coding agents, from the menu-bar icon.
//!
//! The work is done by the bundled `install_hooks.py` (the same script `scripts/install-hooks.sh`
//! runs), so the menu and the command line write identical files. This module finds the script,
//! runs it, and turns its JSON status into menu rows.
use crate::mapping::AgentDef;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AgentStatus {
    /// The agent's config folder exists on this Mac.
    pub present: bool,
    /// Our hooks are in the agent's config.
    pub installed: bool,
}

pub type Status = BTreeMap<String, AgentStatus>;

pub fn parse_status(json: &str) -> Status {
    serde_json::from_str(json).unwrap_or_default()
}

/// One line of the "Install hooks" submenu.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: String,
    pub text: String,
    pub enabled: bool,
    pub installed: bool,
}

/// A row per agent: `[installed]` ones can be removed, present ones installed, the rest are greyed out.
pub fn install_rows(agents: &[AgentDef], status: &Status) -> Vec<Row> {
    agents
        .iter()
        .map(|a| {
            let st = status.get(&a.id).cloned().unwrap_or_default();
            let note = a
                .note
                .as_deref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default();
            let (text, enabled) = if !a.installable() {
                (format!("{}  -{note}", a.label), false)
            } else if st.installed {
                (
                    format!("✓ {}  installed, click to remove{note}", a.label),
                    true,
                )
            } else if st.present {
                (format!("{}  click to install", a.label), true)
            } else {
                (format!("{}  (not found on this Mac)", a.label), false)
            };
            Row {
                id: a.id.clone(),
                text,
                enabled,
                installed: st.installed,
            }
        })
        .collect()
}

/// The agents "Map animations" and "Active source" offer: only those whose hooks are installed.
pub fn installed_agents<'a>(agents: &'a [AgentDef], status: &Status) -> Vec<&'a AgentDef> {
    agents
        .iter()
        .filter(|a| a.installable() && status.get(&a.id).is_some_and(|s| s.installed))
        .collect()
}

/// Where the installer and the hook script are, and how to run them.
#[derive(Debug, Clone)]
pub struct Installer {
    pub python: String,
    pub script: PathBuf,
    pub hook: PathBuf,
    /// Where `~` points for the installer; `None` = the real home folder.
    pub home: Option<PathBuf>,
}

impl Installer {
    /// The installed app keeps both under `~/.deepseek-pet/hooks`; a repo run uses the checkout
    /// (so a dev build never runs the copy an installed app left in the home folder).
    pub fn locate() -> Option<Self> {
        let user = crate::home::user_home_dir().join("hooks");
        let bundled = std::env::current_exe()
            .ok()
            .and_then(|exe| crate::home::bundle_resources(&exe))
            .is_some();
        let (script, hook) = if bundled && user.join("install_hooks.py").is_file() {
            (user.join("install_hooks.py"), user.join("pet-hook.sh"))
        } else {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            (
                root.join("../../../tools/install_hooks.py"),
                root.join("../hooks/pet-hook.sh"),
            )
        };
        if !script.is_file() || !hook.is_file() {
            return None;
        }
        let python = std::env::var("KOTORI_PET_PYTHON")
            .ok()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "/usr/bin/python3".to_string());
        Some(Self {
            python,
            script,
            hook,
            home: None,
        })
    }

    fn run(&self, action: &str, only: Option<&str>) -> io::Result<String> {
        let mut cmd = Command::new(&self.python);
        cmd.arg(&self.script)
            .arg(action)
            .arg("--hook-script")
            .arg(&self.hook);
        if let Some(home) = &self.home {
            cmd.arg("--home").arg(home);
        }
        if let Some(only) = only {
            cmd.arg("--only").arg(only);
        }
        let out = cmd.output()?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(io::Error::other(
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ))
        }
    }

    pub fn status(&self) -> Status {
        self.run("status", None)
            .map(|t| parse_status(&t))
            .unwrap_or_default()
    }

    pub fn set_installed(&self, agent: &str, install: bool) -> io::Result<()> {
        self.run(if install { "install" } else { "uninstall" }, Some(agent))
            .map(|_| ())
    }
}

/// `/usr/bin/python3` on a Mac without the command line tools is a stub that opens an install dialog.
/// Ask `xcode-select` first, which never prompts, so the menu can say what is missing instead.
pub fn python_ready(python: &str) -> bool {
    if python != "/usr/bin/python3" {
        return std::path::Path::new(python).exists();
    }
    Command::new("/usr/bin/xcode-select")
        .arg("-p")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::agents;

    fn status(entries: &[(&str, bool, bool)]) -> Status {
        entries
            .iter()
            .map(|(id, present, installed)| {
                (
                    id.to_string(),
                    AgentStatus {
                        present: *present,
                        installed: *installed,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn ac_0_8_5_install_menu_lists_all_agents() {
        let rows = install_rows(agents(), &Status::new());
        assert_eq!(rows.len(), agents().len());
        for label in [
            "Claude Code",
            "Z.ai ZCode",
            "Codex CLI",
            "Gemini CLI",
            "Qwen Code",
            "Kimi Code",
            "GitHub Copilot CLI",
            "Cursor CLI",
            "OpenCode",
            "DeepSeek Harness",
        ] {
            assert!(
                rows.iter().any(|r| r.text.contains(label)),
                "{label} missing"
            );
        }
    }

    #[test]
    fn ac_0_8_5_menu_shows_installed_state() {
        let st = status(&[
            ("claude-code", true, true),
            ("gemini", true, false),
            ("codex", true, true),
        ]);
        let rows = install_rows(agents(), &st);
        let row = |id: &str| rows.iter().find(|r| r.id == id).unwrap().clone();
        assert!(row("claude-code").text.contains("installed"));
        assert!(row("claude-code").installed && row("claude-code").enabled);
        assert!(row("gemini").text.contains("click to install"));
        assert!(!row("gemini").installed && row("gemini").enabled);
        assert!(
            row("codex").text.contains("approve the hooks"),
            "codex says what to do after installing"
        );
    }

    #[test]
    fn ac_0_8_5_missing_agent_is_disabled() {
        let st = status(&[("claude-code", true, false)]);
        let rows = install_rows(agents(), &st);
        let qwen = rows.iter().find(|r| r.id == "qwen").unwrap();
        assert!(!qwen.enabled && qwen.text.contains("not found"));
        let harness = rows.iter().find(|r| r.id == "deepseek-harness").unwrap();
        assert!(!harness.enabled, "nothing to install for DeepSeek Harness");
        assert!(harness.text.contains("Claude Code hooks"));
    }

    #[test]
    fn ac_0_8_6_mapping_menu_for_every_installed_agent() {
        let st = status(&[
            ("claude-code", true, true),
            ("gemini", true, true),
            ("cursor", true, false),
            ("qwen", false, false),
        ]);
        let ids: Vec<&str> = installed_agents(agents(), &st)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(ids, ["claude-code", "gemini"]);
        assert!(installed_agents(agents(), &Status::new()).is_empty());
    }

    #[test]
    fn ac_0_8_6_unsupported_events_hidden() {
        let get = |id: &str| agents().iter().find(|a| a.id == id).unwrap();
        assert!(!get("zai").events.iter().any(|e| e == "Notification"));
        assert!(!get("codex").events.iter().any(|e| e == "SessionEnd"));
        assert!(get("claude-code").events.iter().any(|e| e == "PostCompact"));
        assert!(get("deepseek-harness").events.is_empty());
    }

    #[test]
    fn ac_0_8_5_status_json_from_the_installer_parses() {
        let st = parse_status(
            r#"{"gemini": {"present": true, "installed": false, "config": "/x"}, "qwen": {"present": false, "installed": false, "config": "/y"}}"#,
        );
        assert_eq!(
            st["gemini"],
            AgentStatus {
                present: true,
                installed: false
            }
        );
        assert!(parse_status("not json").is_empty());
    }

    #[test]
    fn ac_0_8_5_menu_toggle_runs_the_installer() {
        // the real script on a temp home, as the menu would call it
        let Some(mut installer) = Installer::locate() else {
            panic!("installer script not found");
        };
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let home = std::env::temp_dir().join(format!("deepseek-pet-hooks-{n}"));
        std::fs::create_dir_all(home.join(".gemini")).unwrap();
        installer.home = Some(home.clone());
        installer.python = "python3".to_string();
        assert!(installer.status()["gemini"].present);
        installer.set_installed("gemini", true).unwrap();
        assert!(installer.status()["gemini"].installed);
        installer.set_installed("gemini", false).unwrap();
        assert!(!installer.status()["gemini"].installed);
        assert!(installer.set_installed("not-an-agent", true).is_err());
        let _ = std::fs::remove_dir_all(home);
    }
}
