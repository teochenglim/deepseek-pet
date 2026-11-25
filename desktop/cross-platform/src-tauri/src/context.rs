//! What the "Show context" menu entry says. Claude Code does not expose the `/context` screen to
//! other programs, but it hands the numbers behind it (window size, tokens used, cost) to the status
//! line on every refresh. `statusline.rs` saves the latest document as `~/.deepseek-pet/context.json`;
//! this module turns it into text for the pet's speech bubble.
use crate::i18n::tr;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const BAR_WIDTH: usize = 10;

pub fn context_path(home: &Path) -> PathBuf {
    home.join("context.json")
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

fn num(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Tokens counted against the window: fresh input plus both cache kinds (output is not in the window yet).
pub fn used_tokens(window: &Value) -> u64 {
    let usage = window.get("current_usage").unwrap_or(&Value::Null);
    num(usage, "input_tokens")
        + num(usage, "cache_creation_input_tokens")
        + num(usage, "cache_read_input_tokens")
}

/// Percent of the window in use: Claude Code's own figure when present, else computed.
pub fn percent(window: &Value) -> f64 {
    if let Some(p) = window.get("used_percentage").and_then(Value::as_f64) {
        return p.clamp(0.0, 100.0);
    }
    let size = num(window, "context_window_size");
    if size == 0 {
        return 0.0;
    }
    (used_tokens(window) as f64 * 100.0 / size as f64).clamp(0.0, 100.0)
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

pub fn missing_text() -> String {
    tr(
        "No context yet. Install the statusline (menu bar > Install hooks), then start Claude Code.",
        "还没有上下文数据。请先安装状态栏（菜单栏 > 安装钩子），再启动 Claude Code。",
    )
    .to_string()
}

/// The status line's text rows: model and folder, context bar with tokens, session cost.
pub fn status_rows(doc: &Value) -> Vec<String> {
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
    let folder = std::path::Path::new(cwd)
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

/// Bubble text for a saved document; `now` is unix seconds (a parameter so tests are exact).
pub fn summary(doc: &Value, now: u64) -> String {
    let window = doc.get("context_window").unwrap_or(&Value::Null);
    let size = num(window, "context_window_size");
    if size == 0 {
        return tr(
            "Context: Claude Code has not reported any numbers yet.",
            "上下文：Claude Code 还没有报告数据。",
        )
        .to_string();
    }
    let model = doc.get("model").and_then(Value::as_str).unwrap_or("Claude");
    let pct = percent(window);
    let usage = window.get("current_usage").unwrap_or(&Value::Null);
    let mut lines = vec![
        format!("{} · {model}", tr("Context", "上下文")),
        format!(
            "{} {pct:.0}%  {} / {}",
            bar(pct),
            tokens(used_tokens(window)),
            tokens(size)
        ),
        format!(
            "{} {} · {} {} · {} {}",
            tr("input", "输入"),
            tokens(num(usage, "input_tokens")),
            tr("cache read", "缓存读"),
            tokens(num(usage, "cache_read_input_tokens")),
            tr("cache write", "缓存写"),
            tokens(num(usage, "cache_creation_input_tokens")),
        ),
    ];
    let mut tail = Vec::new();
    if let Some(cost) = doc.pointer("/cost/total_cost_usd").and_then(Value::as_f64) {
        tail.push(format!("${cost:.2}"));
    }
    if let Some(updated) = doc.get("updated").and_then(Value::as_u64) {
        tail.push(format!(
            "{} {} {}",
            tr("updated", "更新于"),
            age(now.saturating_sub(updated)),
            tr("ago", "前")
        ));
    }
    if !tail.is_empty() {
        lines.push(tail.join(" · "));
    }
    lines.join("\n")
}

/// The text for the menu entry: the saved document, or a hint when there is none.
pub fn load_text(home: &Path) -> String {
    let doc = std::fs::read_to_string(context_path(home))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match doc {
        Some(doc) => summary(&doc, now),
        None => missing_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc() -> Value {
        json!({
            "updated": 1000, "model": "Opus 5",
            "context_window": {
                "context_window_size": 200000, "used_percentage": 42,
                "current_usage": {"input_tokens": 1000, "cache_read_input_tokens": 83000, "cache_creation_input_tokens": 0}
            },
            "cost": {"total_cost_usd": 0.4213}
        })
    }

    #[test]
    fn ac_0_9_2_summary_shows_model_percent_tokens_cost_and_age() {
        let text = summary(&doc(), 1012);
        assert!(text.contains("Opus 5"), "{text}");
        assert!(text.contains("42%"), "{text}");
        assert!(text.contains("84k / 200k"), "{text}");
        assert!(text.contains("cache read 83k"), "{text}");
        assert!(text.contains("$0.42"), "{text}");
        assert!(text.contains("12s ago"), "{text}");
    }

    #[test]
    fn ac_0_9_2_percent_is_computed_when_claude_code_omits_it() {
        let w = json!({"context_window_size": 1000, "current_usage": {"input_tokens": 250}});
        assert_eq!(percent(&w), 25.0);
        assert_eq!(percent(&json!({})), 0.0);
        assert_eq!(bar(42.0), "████░░░░░░");
        assert_eq!(bar(0.0), "░░░░░░░░░░");
        assert_eq!(bar(250.0 / 2.0), "██████████"); // never wider than the bar
        assert_eq!(tokens(999), "999");
        assert_eq!(tokens(1_250_000), "1.2M");
    }

    #[test]
    fn ac_0_9_5_context_rows_stand_alone_without_the_avatar() {
        // the live payload Claude Code sends (not the saved file `doc()` above)
        let live = json!({
            "model": {"display_name": "Opus 5"},
            "workspace": {"current_dir": "/x/deepseek-pet"},
            "context_window": {"context_window_size": 200000, "used_percentage": 42,
                "current_usage": {"input_tokens": 1000, "cache_read_input_tokens": 83000}},
            "cost": {"total_cost_usd": 0.4213}
        });
        let rows = status_rows(&live);
        assert_eq!(rows.len(), 3);
        assert!(
            rows[0].contains("Opus 5") && rows[0].contains("deepseek-pet"),
            "{rows:?}"
        );
        assert!(
            rows[1].contains("42%") && rows[1].contains("84k/200k"),
            "{rows:?}"
        );
        assert!(rows[2].contains("$0.42"), "{rows:?}");
        assert!(
            rows.iter().all(|r| !r.contains('\u{a0}')),
            "no avatar padding in the rows"
        );
        assert_eq!(status_rows(&json!({}))[1], "context: no data yet");
    }

    #[test]
    fn ac_0_9_2_no_data_gives_a_hint_not_an_error() {
        let home = std::env::temp_dir().join("deepseek-pet-ctx-none");
        let _ = std::fs::remove_dir_all(&home);
        assert!(load_text(&home).contains("Install the statusline"));
        assert!(summary(&json!({"context_window": {}}), 0).contains("not reported"));
    }
}
