//! English / Chinese switch for the menus and the speech bubble, chosen from the menu-bar icon
//! and kept in `settings.json` (`language`). One process-wide flag, so the menu builders and the
//! event watcher need no extra plumbing.
use std::sync::atomic::{AtomicBool, Ordering};

static ZH: AtomicBool = AtomicBool::new(false);

pub fn set_zh(on: bool) {
    ZH.store(on, Ordering::Relaxed);
}

pub fn is_zh() -> bool {
    ZH.load(Ordering::Relaxed)
}

/// `"zh"` means Chinese; anything else (or nothing) is English.
pub fn is_zh_code(code: Option<&str>) -> bool {
    code == Some("zh")
}

/// Pick the Chinese text when Chinese is on.
pub fn tr<'a>(en: &'a str, zh: &'a str) -> &'a str {
    if is_zh() {
        zh
    } else {
        en
    }
}

/// Chinese for the dialogue lines the hooks send; an unknown line is shown as it came.
pub fn dialogue(text: &str) -> String {
    if !is_zh() {
        return text.to_string();
    }
    let zh = match text {
        "Hi!" => "欢迎！",
        "Thinking..." => "思考中...",
        "Working..." => "工作中...",
        "Done!" => "完成啦！",
        "Something went wrong" => "出错了",
        "Heads up" => "注意啦",
        "Needs your approval" => "需要你的批准",
        "Needs your choice" => "需要你来选择",
        "Tidying memory..." => "整理记忆中...",
        "Back to work" => "继续工作",
        "See you!" => "再见！",
        other => other,
    };
    zh.to_string()
}

/// Chinese for a clip category of the catalog.
pub fn category(en: &str) -> String {
    if !is_zh() {
        return en.to_string();
    }
    let zh = match en {
        "Festivals" => "节日",
        "Play and hobbies" => "玩耍与爱好",
        "Moods and movement" => "心情与动作",
        "Meals and snacks" => "用餐与零食",
        "Work" => "工作",
        "Magic and animals" => "魔法与动物",
        "Balance" => "平衡",
        "Click reactions" => "点击回应",
        "Muttering" => "碎碎念",
        other => other,
    };
    zh.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // the flag is process-wide, so the tests that flip it run one at a time
    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn ac_0_8_10_english_is_unchanged() {
        let _g = LOCK.lock().unwrap();
        set_zh(false);
        assert_eq!(dialogue("Hi!"), "Hi!");
        assert_eq!(category("Work"), "Work");
        assert_eq!(tr("Quit", "退出"), "Quit");
    }

    #[test]
    fn ac_0_8_10_chinese_translates_greeting_and_menu_text() {
        let _g = LOCK.lock().unwrap();
        set_zh(true);
        assert_eq!(dialogue("Hi!"), "欢迎！");
        assert_eq!(dialogue("something unknown"), "something unknown");
        assert_eq!(category("Work"), "工作");
        assert_eq!(tr("Quit", "退出"), "退出");
        set_zh(false);
    }

    #[test]
    fn ac_0_8_10_language_code_parses() {
        assert!(is_zh_code(Some("zh")));
        assert!(!is_zh_code(Some("en")));
        assert!(!is_zh_code(None));
    }
}
