//! Claude Code status line: the pet's avatar next to the model, context use and cost.
//!
//! Claude Code runs the configured command after every turn and pipes a JSON document to it. The
//! command is this app's own binary (`deepseek-pet --statusline`, reached through the stable link
//! `~/.deepseek-pet/bin/statusline`), so nothing else has to be installed on the Mac: no Python,
//! no jq, no Node. Each run also saves the document as `~/.deepseek-pet/context.json` for the
//! "Show context" menu entry (`context.rs`).
use crate::context::{self, bar, percent, tokens, used_tokens};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// The avatar as 24-bit-colour quadrant-block text; regenerate with `tools/make_statusline_logo.py`.
const LOGO: &str = include_str!("../../hooks/statusline-logo.ansi");
/// Claude Code trims leading plain spaces from status lines; a no-break space survives.
const NBSP: char = '\u{a0}';
pub const MARKER: &str = "# deepseek-pet";
pub const MENU_ID: &str = "claude-statusline";

// ── rendering ───────────────────────────────────────────────────────────────────────────────

fn visible_width(line: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for c in line.chars() {
        match (in_escape, c) {
            (false, '\x1b') => in_escape = true,
            (true, 'm') => in_escape = false,
            (false, _) => width += 1,
            _ => {}
        }
    }
    width
}

fn text_rows(doc: &Value) -> Vec<String> {
    let window = doc.get("context_window").unwrap_or(&Value::Null);
    let model = doc
        .pointer("/model/display_name")
        .and_then(Value::as_str)
        .unwrap_or("Claude");
    let cwd = doc
        .pointer("/workspace/current_dir")
        .or_else(|| doc.get("cwd"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let folder = Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let mut rows = vec![format!("\x1b[1m{model}\x1b[0m  {folder}")
        .trim_end()
        .to_string()];
    let size = window
        .get("context_window_size")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if size > 0 {
        let pct = percent(window);
        rows.push(format!(
            "context {} {pct:.0}%  {}/{}",
            bar(pct),
            tokens(used_tokens(window)),
            tokens(size)
        ));
    } else {
        rows.push("context: no data yet".to_string());
    }
    if let Some(cost) = doc.pointer("/cost/total_cost_usd").and_then(Value::as_f64) {
        rows.push(format!("${cost:.2} this session"));
    }
    rows
}

/// The avatar with the text rows beside it, vertically centred.
pub fn render(doc: &Value) -> String {
    let rows = text_rows(doc);
    let logo: Vec<String> = LOGO
        .lines()
        .map(|l| l.replace(' ', &NBSP.to_string()))
        .collect();
    if logo.is_empty() {
        return rows.join("\n");
    }
    let width = logo.iter().map(|l| visible_width(l)).max().unwrap_or(0);
    let skip = logo.len().saturating_sub(rows.len()) / 2;
    logo.iter()
        .enumerate()
        .map(|(i, line)| {
            let text = i
                .checked_sub(skip)
                .and_then(|j| rows.get(j))
                .map_or("", |s| s.as_str());
            let pad = NBSP.to_string().repeat(width - visible_width(line));
            format!("{line}{pad}  {text}").trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ── saving the numbers for the menu ─────────────────────────────────────────────────────────

/// The part of Claude Code's document the menu needs, stamped with the time.
pub fn to_saved(doc: &Value, now: u64) -> Value {
    json!({
        "updated": now,
        "session_id": doc.get("session_id"),
        "model": doc.pointer("/model/display_name").or_else(|| doc.pointer("/model/id")),
        "cwd": doc.pointer("/workspace/current_dir").or_else(|| doc.get("cwd")),
        "context_window": doc.get("context_window").cloned().unwrap_or(Value::Null),
        "cost": {"total_cost_usd": doc.pointer("/cost/total_cost_usd")},
    })
}

/// Write-then-rename so the menu never reads half a file.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

pub fn save(home: &Path, doc: &Value, now: u64) -> io::Result<()> {
    write_atomic(
        &context::context_path(home),
        &to_saved(doc, now).to_string(),
    )
}

/// `deepseek-pet --statusline`: read the document, save it, print the status line. Never fails.
pub fn run() -> i32 {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let doc = serde_json::from_str::<Value>(&input)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = save(&crate::home::user_home_dir(), &doc, now);
    let _ = writeln!(io::stdout(), "{}", render(&doc));
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
    write_atomic(settings, &(text + "\n"))
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

    const BLOCKS: &str = "▘▝▀▖▌▞▛▗▚▐▜▄▙▟█";

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
    fn ac_0_9_1_statusline_shows_avatar_and_numbers() {
        let out = render(&payload());
        assert!(
            out.chars().any(|c| BLOCKS.contains(c)),
            "avatar blocks missing"
        );
        assert!(out.contains("\x1b[38;2;"), "avatar colour missing");
        for want in ["Opus 5", "deepseek-pet", "42%", "84k/200k", "$0.42"] {
            assert!(out.contains(want), "{want} missing in {out}");
        }
        assert!(out.lines().count() >= 3, "avatar is several rows tall");
        assert!(
            out.lines().all(|l| !l.starts_with(' ')),
            "leading plain spaces would be trimmed by Claude Code"
        );
    }

    #[test]
    fn ac_0_9_1_broken_payload_still_prints_the_avatar() {
        let out = render(&json!({}));
        assert!(out.chars().any(|c| BLOCKS.contains(c)));
        assert!(out.contains("context: no data yet"));
    }

    #[test]
    fn ac_0_9_1_each_run_saves_the_numbers_for_the_menu() {
        let home = temp("save");
        save(&home, &payload(), 1234).unwrap();
        let saved: Value =
            serde_json::from_str(&std::fs::read_to_string(context::context_path(&home)).unwrap())
                .unwrap();
        assert_eq!(saved["updated"], 1234);
        assert_eq!(saved["model"], "Opus 5");
        assert_eq!(saved["context_window"]["context_window_size"], 200000);
        // the menu reads exactly what the status line wrote
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
