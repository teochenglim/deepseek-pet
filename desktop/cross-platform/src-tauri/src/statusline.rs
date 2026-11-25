//! Claude Code status line: just the pet's avatar, a message-of-the-day line with no text.
//!
//! Claude Code runs the configured command after every turn and pipes a JSON document to it. The
//! command is this app's own binary (`deepseek-pet --statusline`, reached through the stable link
//! `~/.deepseek-pet/bin/statusline`), so nothing else has to be installed on the Mac: no Python,
//! no jq, no Node. The numbers in that document are not shown here: each run hands them to
//! `context.rs` (the separate "Show Claude context" feature) and prints only the avatar.
use crate::context;
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// The avatar as 24-bit-colour block text; regenerate with `tools/make_statusline_logo.py`.
const LOGO: &str = include_str!("../../hooks/statusline-logo.ansi");
/// Claude Code trims leading plain spaces from status lines; a no-break space survives.
const NBSP: char = '\u{a0}';
pub const MARKER: &str = "# deepseek-pet";
pub const MENU_ID: &str = "claude-statusline";

// ── rendering ───────────────────────────────────────────────────────────────────────────────

/// The avatar, one string per text row (transparent cells are no-break spaces).
pub fn avatar_lines() -> Vec<String> {
    LOGO.lines()
        .map(|l| l.replace(' ', &NBSP.to_string()))
        .collect()
}

/// What the status line prints: the avatar and nothing else.
pub fn render() -> String {
    avatar_lines().join("\n")
}

/// One run of the command: keep the numbers for "Show Claude context" (silently), print the avatar.
pub fn handle(input: &str, home: &Path, now: u64) -> String {
    let doc = serde_json::from_str::<Value>(input)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let _ = context::save(home, &doc, now);
    render()
}

/// `deepseek-pet --statusline`: Claude Code's status line command. Never fails.
pub fn run() -> i32 {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = writeln!(
        io::stdout(),
        "{}",
        handle(&input, &crate::home::user_home_dir(), now)
    );
    0
}

// ── wiring it into Claude Code's settings.json ──────────────────────────────────────────────

pub fn claude_settings_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude/settings.json")
}

/// The stable path Claude Code runs; a symlink to the app binary.
pub fn link_path(home: &Path) -> PathBuf {
    home.join("bin").join("statusline")
}

/// Point the stable link at `exe` (replacing an older one, e.g. after an upgrade moved the app).
pub fn ensure_link(home: &Path, exe: &Path) -> io::Result<()> {
    let link = link_path(home);
    if std::fs::read_link(&link).is_ok_and(|target| target == exe) {
        return Ok(());
    }
    if let Some(dir) = link.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::symlink_metadata(&link).is_ok() {
        std::fs::remove_file(&link)?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(exe, &link)
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(exe, &link).map(|_| ())
    }
}

/// The shell line stored in settings.json (the marker lets us recognise our own entry later).
pub fn command_line(home: &Path) -> String {
    let path = link_path(home).to_string_lossy().replace('\'', "'\\''");
    format!("'{path}' --statusline {MARKER}")
}

fn is_ours(line: &Value) -> bool {
    line.get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.trim_end().ends_with(MARKER))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    /// No `~/.claude`: Claude Code is not installed here.
    NotFound,
    /// Claude Code is there with no status line of ours.
    Available,
    /// Ours is set.
    Installed,
    /// The user has their own status line; never replaced.
    Foreign,
}

fn read_settings(path: &Path) -> io::Result<Value> {
    match std::fs::read_to_string(path) {
        Ok(t) if t.trim().is_empty() => Ok(json!({})),
        Ok(t) => serde_json::from_str::<Value>(&t)
            .ok()
            .filter(Value::is_object)
            .ok_or_else(|| io::Error::other(format!("{} is not a JSON object", path.display()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e),
    }
}

pub fn state(settings: &Path) -> State {
    let claude_here = settings.parent().is_some_and(Path::exists);
    match read_settings(settings) {
        Ok(doc) => match doc.get("statusLine") {
            Some(line) if is_ours(line) => State::Installed,
            Some(line) if !line.is_null() => State::Foreign,
            _ if claude_here => State::Available,
            _ => State::NotFound,
        },
        // an unreadable file is left alone, like a foreign status line
        Err(_) => State::Foreign,
    }
}

/// Set (`true`) or remove (`false`) our status line. A file we cannot parse, and a status line that
/// is not ours, are never touched. The first change keeps `settings.json.deepseek-pet.bak`.
pub fn set_installed(settings: &Path, home: &Path, exe: &Path, install: bool) -> io::Result<()> {
    let mut doc = read_settings(settings)?;
    let current = doc.get("statusLine").cloned().unwrap_or(Value::Null);
    if install {
        if !current.is_null() && !is_ours(&current) {
            return Err(io::Error::other(
                "Claude Code already has a status line; left alone",
            ));
        }
        ensure_link(home, exe)?;
        doc["statusLine"] = json!({"type": "command", "command": command_line(home)});
    } else {
        if !is_ours(&current) {
            return Ok(());
        }
        if let Some(map) = doc.as_object_mut() {
            map.remove("statusLine");
        }
    }
    let bak = settings.with_file_name("settings.json.deepseek-pet.bak");
    if settings.is_file() && !bak.exists() {
        std::fs::copy(settings, &bak)?;
    }
    let text = serde_json::to_string_pretty(&doc).map_err(io::Error::other)?;
    crate::home::write_atomic(settings, &(text + "\n"))
}

/// Menu row for the "Install hooks" submenu: (text, enabled, installed).
pub fn menu_row(state: State) -> (String, bool, bool) {
    use crate::i18n::tr;
    match state {
        State::Installed => (
            format!(
                "✓ {}  {}",
                tr("Claude Code statusline", "Claude Code 状态栏"),
                tr("installed, click to remove", "已安装，点击移除")
            ),
            true,
            true,
        ),
        State::Available => (
            format!(
                "{}  {}",
                tr("Claude Code statusline", "Claude Code 状态栏"),
                tr("click to install", "点击安装")
            ),
            true,
            false,
        ),
        State::Foreign => (
            format!(
                "{}  ({})",
                tr("Claude Code statusline", "Claude Code 状态栏"),
                tr("you already have a status line", "你已有自己的状态栏")
            ),
            false,
            false,
        ),
        State::NotFound => (
            format!(
                "{}  ({})",
                tr("Claude Code statusline", "Claude Code 状态栏"),
                tr("not found on this Mac", "未在此 Mac 上找到")
            ),
            false,
            false,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(label: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("deepseek-pet-sl-{label}-{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const BLOCKS: &str = "▀▄▌▐█";

    fn payload() -> Value {
        json!({
            "session_id": "s1",
            "model": {"display_name": "Opus 5"},
            "workspace": {"current_dir": "/x/deepseek-pet"},
            "context_window": {"context_window_size": 200000, "used_percentage": 42,
                "current_usage": {"input_tokens": 1000, "cache_read_input_tokens": 83000}},
            "cost": {"total_cost_usd": 0.4213}
        })
    }

    #[test]
    fn ac_0_9_7_statusline_prints_only_the_avatar() {
        let out = handle(&payload().to_string(), &temp("only"), 1);
        assert_eq!(out, avatar_lines().join("\n"));
        assert!(out.lines().count() >= 8, "avatar is several rows tall");
        for text in ["Opus 5", "deepseek-pet", "42%", "84k", "$0.42", "context"] {
            assert!(!out.contains(text), "{text} must not be printed");
        }
        assert!(
            out.lines().all(|l| !l.starts_with(' ')),
            "leading plain spaces would be trimmed by Claude Code"
        );
    }

    #[test]
    fn ac_0_9_1_avatar_uses_only_widely_measured_block_characters() {
        // Claude Code shifted the text rows when the avatar held other quadrant glyphs (v0.9.1)
        for c in LOGO
            .chars()
            .filter(|c| ('\u{2580}'..='\u{259f}').contains(c))
        {
            assert!(BLOCKS.contains(c), "{c} may be mis-measured by Claude Code");
        }
    }

    #[test]
    fn ac_0_9_5_avatar_stands_alone_without_any_text() {
        let text = avatar_lines().join("\n");
        assert!(!text.contains("context") && !text.contains('$'));
        assert!(text.chars().any(|c| BLOCKS.contains(c)));
    }

    #[test]
    fn ac_0_9_1_broken_payload_still_prints_the_avatar() {
        let home = temp("broken");
        for input in ["", "not json", "[1,2]"] {
            let out = handle(input, &home, 1);
            assert!(out.chars().any(|c| BLOCKS.contains(c)), "{input:?}");
        }
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn ac_0_9_7_statusline_still_records_the_numbers_silently() {
        let home = temp("save");
        let out = handle(&payload().to_string(), &home, 1234);
        assert!(!out.contains("42%"), "recorded, not printed");
        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(context::context_path(&home)).unwrap())
                .unwrap();
        assert_eq!(saved["updated"], 1234);
        assert_eq!(saved["model"], "Opus 5");
        assert_eq!(saved["context_window"]["context_window_size"], 200000);
        // "Show Claude context" reads exactly what the status line recorded
        assert!(context::summary(&saved, 1234).contains("42%"));
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn ac_0_9_3_install_and_remove_keep_the_users_other_settings() {
        let root = temp("install");
        let (home, claude) = (root.join("pet"), root.join(".claude"));
        std::fs::create_dir_all(&claude).unwrap();
        let settings = claude.join("settings.json");
        std::fs::write(&settings, r#"{"zeta": 1, "alpha": {"b": 2}}"#).unwrap();
        assert_eq!(state(&settings), State::Available);

        set_installed(&settings, &home, Path::new("/Applications/Pet"), true).unwrap();
        assert_eq!(state(&settings), State::Installed);
        let text = std::fs::read_to_string(&settings).unwrap();
        assert!(
            text.find("zeta").unwrap() < text.find("alpha").unwrap(),
            "key order kept"
        );
        assert!(text.contains("--statusline # deepseek-pet"));
        assert!(claude.join("settings.json.deepseek-pet.bak").is_file());
        assert!(std::fs::symlink_metadata(link_path(&home)).is_ok());

        set_installed(&settings, &home, Path::new("/Applications/Pet"), false).unwrap();
        assert_eq!(state(&settings), State::Available);
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(after, json!({"zeta": 1, "alpha": {"b": 2}}));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ac_0_9_3_a_status_line_of_the_users_own_is_never_replaced() {
        let root = temp("foreign");
        let claude = root.join(".claude");
        std::fs::create_dir_all(&claude).unwrap();
        let settings = claude.join("settings.json");
        let mine = r#"{"statusLine":{"type":"command","command":"my-line.sh"}}"#;
        std::fs::write(&settings, mine).unwrap();
        assert_eq!(state(&settings), State::Foreign);
        assert!(set_installed(&settings, &root, Path::new("/x"), true).is_err());
        set_installed(&settings, &root, Path::new("/x"), false).unwrap(); // not ours: no-op
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), mine);
        let (text, enabled, _) = menu_row(State::Foreign);
        assert!(!enabled && text.contains("already have"));
        // a file that is not JSON is not overwritten either
        std::fs::write(&settings, "{ not json").unwrap();
        assert!(set_installed(&settings, &root, Path::new("/x"), true).is_err());
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), "{ not json");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ac_0_9_3_link_follows_the_app_when_it_moves() {
        let home = temp("link");
        ensure_link(&home, Path::new("/old/Pet")).unwrap();
        ensure_link(&home, Path::new("/new/Pet")).unwrap();
        assert_eq!(
            std::fs::read_link(link_path(&home)).unwrap(),
            PathBuf::from("/new/Pet")
        );
        assert!(command_line(&home).ends_with("--statusline # deepseek-pet"));
        let _ = std::fs::remove_dir_all(home);
    }
}
