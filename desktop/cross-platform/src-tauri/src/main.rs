// 在 Windows release 构建中阻止额外的控制台窗口出现，切勿删除！！
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Claude Code's status line runs this same binary; answer before any window machinery starts
    if std::env::args().nth(1).as_deref() == Some("--statusline") {
        std::process::exit(deepseek_pet_lib::run_statusline());
    }
    deepseek_pet_lib::run()
}
