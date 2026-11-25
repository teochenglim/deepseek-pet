//! "Show Claude context": what the speech bubble says when you ask for the context.
//!
//! Claude Code does not let other programs read its `/context` screen, but every session is logged
//! as a transcript (`~/.claude/projects/<project>/<session>.jsonl`) and each assistant line carries
//! the token usage of that request. The newest transcript's last assistant line therefore says how
//! much of the window the conversation fills right now. Nothing is installed or changed in Claude
//! Code for this, and nothing is stored: the file is read when you ask.
//!
//! The window size is not in the transcript, so it is assumed and shown with a `~`: 1M tokens for a
//! `1m` model name or when more than 200k are already in use (it cannot be a 200k window then),
//! otherwise 200k.
use crate::i18n::tr;
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const BAR_WIDTH: usize = 10;
/// Only the end of a transcript is read: the last assistant line is always near it.
const TAIL_BYTES: u64 = 512 * 1024;
const WINDOW_STANDARD: u64 = 200_000;
const WINDOW_LONG: u64 = 1_000_000;

/// Token use of the newest request in the newest Claude Code transcript.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Usage {
    pub model: String,
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
    /// Window size assumed from the model name.
    pub window: u64,
    /// When the transcript was last written (unix seconds).
    pub updated: u64,
}

impl Usage {
    /// What the conversation holds now: the request's input (fresh + both cache kinds) plus its answer.
    pub fn used(&self) -> u64 {
        self.input + self.cache_write + self.cache_read + self.output
    }

    pub fn percent(&self) -> f64 {
        if self.window == 0 {
            return 0.0;
        }
        (self.used() as f64 * 100.0 / self.window as f64).clamp(0.0, 100.0)
    }
}

/// `~/.claude/projects`.
pub fn claude_projects_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude/projects")
}

/// 84_300 -> "84k", 1_250_000 -> "1.2M".
pub fn tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", (n as f64 / 1_000.0).round() as u64)
    } else {
        n.to_string()
    }
}

pub fn bar(pct: f64) -> String {
    let filled = ((pct / 100.0 * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
    format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled))
}

fn age(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{} min", seconds / 60),
        _ => format!("{} h", seconds / 3600),
    }
}

/// A `1m` model variant (e.g. `sonnet[1m]`) has the long window, and so does any conversation that
/// already holds more than the standard window; everything else is assumed standard.
pub fn window_for(model: &str, used: u64) -> u64 {
    if used > WINDOW_STANDARD || model.to_lowercase().contains("1m") {
        WINDOW_LONG
    } else {
        WINDOW_STANDARD
    }
}

fn num(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// The last assistant line with usage in `text` (JSON lines, oldest first). Lines that are cut off,
/// not JSON, not from the assistant, or synthetic (no real request) are skipped.
pub fn parse_last_usage(text: &str) -> Option<Usage> {
    text.lines().rev().find_map(|line| {
        let entry: Value = serde_json::from_str(line).ok()?;
        if entry.get("type").and_then(Value::as_str) != Some("assistant") {
            return None;
        }
        let message = entry.get("message")?;
        let usage = message.get("usage").filter(|u| u.is_object())?;
        let model = message.get("model").and_then(Value::as_str).unwrap_or("");
        if model.is_empty() || model.starts_with('<') {
            return None;
        }
        let mut found = Usage {
            model: model.to_string(),
            input: num(usage, "input_tokens"),
            cache_write: num(usage, "cache_creation_input_tokens"),
            cache_read: num(usage, "cache_read_input_tokens"),
            output: num(usage, "output_tokens"),
            window: 0,
            updated: 0,
        };
        found.window = window_for(model, found.used());
        Some(found)
    })
}

/// The most recently written `*.jsonl` directly inside any project folder, with its mtime in unix seconds.
pub fn latest_transcript(projects: &Path) -> Option<(PathBuf, u64)> {
    let mut best: Option<(PathBuf, SystemTime)> = None;
    for project in std::fs::read_dir(projects).ok()?.flatten() {
        let Ok(files) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(modified) = file.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(_, t)| modified > *t) {
                best = Some((path, modified));
            }
        }
    }
    let (path, modified) = best?;
    let secs = modified
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some((path, secs))
}

/// The last `TAIL_BYTES` of a file, starting at a line boundary.
fn read_tail(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let cut = len > TAIL_BYTES;
    if cut {
        file.seek(SeekFrom::Start(len - TAIL_BYTES)).ok()?;
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Some(if cut {
        text.split_once('\n')?.1.to_string() // drop the partial first line
    } else {
        text
    })
}

/// Newest transcript under `projects` and what its last request used.
pub fn load(projects: &Path) -> Option<Usage> {
    let (path, updated) = latest_transcript(projects)?;
    let mut usage = parse_last_usage(&read_tail(&path)?)?;
    usage.updated = updated;
    Some(usage)
}

pub fn missing_text() -> String {
    tr(
        "No Claude context yet. Start Claude Code and send a message.",
        "还没有 Claude 上下文数据。请启动 Claude Code 并发送一条消息。",
    )
    .to_string()
}

/// Bubble text; `now` is unix seconds (a parameter so tests are exact).
pub fn summary(usage: &Usage, now: u64) -> String {
    let pct = usage.percent();
    [
        format!(
            "{} · {}",
            tr("Claude context", "Claude 上下文"),
            usage.model
        ),
        format!(
            "{} {pct:.0}%  {} / ~{}",
            bar(pct),
            tokens(usage.used()),
            tokens(usage.window)
        ),
        format!(
            "{} {} · {} {} · {} {} · {} {}",
            tr("input", "输入"),
            tokens(usage.input),
            tr("cache read", "缓存读"),
            tokens(usage.cache_read),
            tr("cache write", "缓存写"),
            tokens(usage.cache_write),
            tr("output", "输出"),
            tokens(usage.output),
        ),
        format!(
            "{} {} {}",
            tr("updated", "更新于"),
            age(now.saturating_sub(usage.updated)),
            tr("ago", "前")
        ),
    ]
    .join("\n")
}

/// The text for the menu entry: the newest transcript's numbers, or a hint when there is none.
pub fn load_text(projects: &Path) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match load(projects) {
        Some(usage) => summary(&usage, now),
        None => missing_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: &str, model: &str, input: u64, read: u64, write: u64, output: u64) -> String {
        serde_json::json!({
            "type": kind, "cwd": "/x",
            "message": {"model": model, "usage": {
                "input_tokens": input, "cache_read_input_tokens": read,
                "cache_creation_input_tokens": write, "output_tokens": output}}
        })
        .to_string()
    }

    fn temp(label: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("deepseek-pet-ctx-{label}-{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ac_0_9_2_summary_shows_model_percent_tokens_cache_and_age() {
        let usage = Usage {
            model: "claude-sonnet-5-5".into(),
            input: 1000,
            cache_read: 83000,
            cache_write: 0,
            output: 2000,
            window: 200_000,
            updated: 1000,
        };
        let text = summary(&usage, 1012);
        assert!(text.contains("claude-sonnet-5-5"), "{text}");
        assert!(text.contains("43%"), "{text}");
        assert!(text.contains("86k / ~200k"), "{text}");
        assert!(text.contains("cache read 83k"), "{text}");
        assert!(text.contains("output 2k"), "{text}");
        assert!(text.contains("12s ago"), "{text}");
    }

    #[test]
    fn ac_0_9_2_percent_bar_and_tokens_stay_in_range() {
        let mut u = Usage {
            input: 250,
            window: 1000,
            ..Usage::default()
        };
        assert_eq!(u.percent(), 25.0);
        u.input = 5000;
        assert_eq!(u.percent(), 100.0);
        assert_eq!(Usage::default().percent(), 0.0);
        assert_eq!(bar(42.0), "████░░░░░░");
        assert_eq!(bar(0.0), "░░░░░░░░░░");
        assert_eq!(bar(125.0), "██████████"); // never wider than the bar
        assert_eq!(tokens(999), "999");
        assert_eq!(tokens(1_250_000), "1.2M");
    }

    #[test]
    fn ac_0_9_2_no_data_gives_a_hint_not_an_error() {
        let projects = temp("none");
        assert!(load_text(&projects).contains("No Claude context yet"));
        assert!(load_text(&projects.join("missing")).contains("No Claude context yet"));
        let _ = std::fs::remove_dir_all(projects);
    }

    #[test]
    fn ac_0_9_8_reads_the_last_usage_of_the_newest_transcript() {
        let projects = temp("newest");
        let (old, new) = (projects.join("a"), projects.join("b"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(
            old.join("old.jsonl"),
            line("assistant", "claude-opus-5-5", 9, 9, 9, 9),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let body = [
            line("assistant", "claude-sonnet-5-5", 1, 10_000, 500, 20),
            line("user", "", 0, 0, 0, 0),
            line("assistant", "claude-sonnet-5-5", 3, 80_000, 1_000, 400),
        ]
        .join("\n");
        std::fs::write(new.join("new.jsonl"), body).unwrap();
        let usage = load(&projects).unwrap();
        assert_eq!(usage.model, "claude-sonnet-5-5");
        assert_eq!((usage.input, usage.cache_read), (3, 80_000));
        assert_eq!((usage.cache_write, usage.output), (1_000, 400));
        assert_eq!(usage.used(), 81_403);
        assert!(usage.updated > 0);
        let _ = std::fs::remove_dir_all(projects);
    }

    #[test]
    fn ac_0_9_8_skips_synthetic_cut_off_and_non_assistant_lines() {
        let text = [
            line("assistant", "claude-sonnet-5-5", 5, 6, 7, 8),
            line("assistant", "<synthetic>", 0, 0, 0, 0),
            line("user", "claude-sonnet-5-5", 99, 99, 99, 99),
            "{\"type\":\"assistant\",\"message\":{\"mod".to_string(),
            "not json at all".to_string(),
        ]
        .join("\n");
        let usage = parse_last_usage(&text).unwrap();
        assert_eq!((usage.input, usage.cache_read, usage.output), (5, 6, 8));
        assert!(parse_last_usage("").is_none());
        assert!(parse_last_usage(&line("user", "m", 1, 1, 1, 1)).is_none());
        // the window comes from the model name
        assert_eq!(window_for("claude-sonnet-5-5", 50_000), 200_000);
        assert_eq!(window_for("sonnet[1m]", 50_000), 1_000_000);
        // more than 200k in use can only be a long window
        assert_eq!(window_for("claude-sonnet-5-5", 302_000), 1_000_000);
    }

    #[test]
    fn ac_0_9_6_missing_data_hint_says_how_to_get_it() {
        let hint = missing_text();
        assert!(hint.contains("Claude context"), "{hint}");
        assert!(hint.contains("send a message"), "{hint}");
    }
}
