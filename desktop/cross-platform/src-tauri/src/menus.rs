//! Native menus shared by the menu-bar icon and the pet's own right-click "Play animation" popup.
//! Menu ids carry a prefix so the two sources never trigger each other's handlers:
//! the tray uses plain ids ("show", "play:c001"), the popup uses "pop:" ("pop:play:c001").
use crate::catalog::{self, Clip};
use std::sync::Arc;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Wry};

/// Clips available to the menus; managed as app state so handlers can look them up.
pub struct PlayCatalog(pub Arc<Vec<Clip>>);

pub const POPUP_PREFIX: &str = "pop:";

#[derive(Debug, PartialEq)]
pub enum Action {
    Show,
    Hide,
    Quit,
    /// A clip id, or "random".
    Play(String),
}

/// Turn a menu id into an action, but only when it carries `prefix` ("" for the tray).
pub fn parse_action(id: &str, prefix: &str) -> Option<Action> {
    let rest = id.strip_prefix(prefix)?;
    match rest {
        "show" => Some(Action::Show),
        "hide" => Some(Action::Hide),
        "quit" => Some(Action::Quit),
        _ => rest
            .strip_prefix("play:")
            .map(|wanted| Action::Play(wanted.to_string())),
    }
}

/// "Random animation" plus one submenu per category, ids built with `prefix`.
pub fn play_entries(
    app: &AppHandle,
    clips: &[Clip],
    prefix: &str,
) -> tauri::Result<(MenuItem<Wry>, Vec<Submenu<Wry>>)> {
    let random = MenuItem::with_id(
        app,
        format!("{prefix}play:random"),
        "Random animation",
        !clips.is_empty(),
        None::<&str>,
    )?;
    let mut categories = Vec::new();
    for (category, group) in catalog::group_by_category(clips) {
        let sub = Submenu::with_id(app, format!("{prefix}cat:{category}"), &category, true)?;
        for clip in group {
            sub.append(&MenuItem::with_id(
                app,
                format!("{prefix}play:{}", clip.id),
                &clip.title,
                true,
                None::<&str>,
            )?)?;
        }
        categories.push(sub);
    }
    Ok((random, categories))
}

/// Right-click menu on the pet: Random animation, one submenu per category, then Hide and Quit.
/// Everything is on the first screen, so choosing a clip needs no extra click.
pub fn pet_popup(app: &AppHandle, clips: &[Clip]) -> tauri::Result<Menu<Wry>> {
    let (random, categories) = play_entries(app, clips, POPUP_PREFIX)?;
    let menu = Menu::new(app)?;
    menu.append(&random)?;
    let items: Vec<&dyn IsMenuItem<Wry>> = categories
        .iter()
        .map(|c| c as &dyn IsMenuItem<Wry>)
        .collect();
    menu.append_items(&items)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        format!("{POPUP_PREFIX}hide"),
        "Hide for now",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        format!("{POPUP_PREFIX}quit"),
        "Quit",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

/// Carry out a parsed menu action.
pub fn run(app: &AppHandle, action: Action) {
    match action {
        Action::Show => crate::commands::show_pet(app),
        Action::Hide => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        Action::Quit => crate::commands::shutdown(app),
        Action::Play(wanted) => {
            let Some(state) = app.try_state::<PlayCatalog>() else {
                return;
            };
            let clips = &state.0;
            let clip = if wanted == "random" {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                catalog::pick_index(clips.len(), seed).map(|i| &clips[i])
            } else {
                clips.iter().find(|c| c.id == wanted)
            };
            if let Some(clip) = clip {
                crate::commands::show_pet(app);
                let _ = app.emit("play-animation", clip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_0_6_5_menu_ids_parse() {
        assert_eq!(parse_action("show", ""), Some(Action::Show));
        assert_eq!(parse_action("quit", ""), Some(Action::Quit));
        assert_eq!(
            parse_action("play:c042", ""),
            Some(Action::Play("c042".into()))
        );
        assert_eq!(
            parse_action("pop:play:random", POPUP_PREFIX),
            Some(Action::Play("random".into()))
        );
        assert_eq!(parse_action("pop:hide", POPUP_PREFIX), Some(Action::Hide));
        assert_eq!(parse_action("cat:Work", ""), None);
    }

    #[test]
    fn ac_0_6_5_tray_and_popup_do_not_trigger_each_other() {
        // a popup click must not be handled by the tray handler, and vice versa
        assert_eq!(parse_action("pop:play:c001", ""), None);
        assert_eq!(parse_action("pop:quit", ""), None);
        assert_eq!(parse_action("play:c001", POPUP_PREFIX), None);
        assert_eq!(parse_action("quit", POPUP_PREFIX), None);
    }
}
