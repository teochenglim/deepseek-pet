//! Native menus shared by the menu-bar icon and the pet's own right-click "Play animation" popup.
//! Menu ids carry a prefix so the two sources never trigger each other's handlers:
//! the tray uses plain ids ("show", "play:c001"), the popup uses "pop:" ("pop:play:c001").
use crate::aggregator::ActivityAggregator;
use crate::catalog::{self, Clip};
use crate::hooks::{self, Installer, Status};
use crate::mapping::{self, AgentDef, Choice, Mapping, MappingStore, MAPPABLE_STATES};
use crate::settings::SettingsStore;
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
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
    /// Set (`Some`) or clear (`None`, back to the default) one event's animation.
    MapSet {
        agent: String,
        event: String,
        choice: Option<Choice>,
    },
    /// Map the clip that was played last to one event.
    MapLast {
        agent: String,
        event: String,
    },
    MapReset,
    /// Let one source drive the pet (`Some(agent id)`), or all of them (`None`).
    Source(Option<String>),
    /// Install (`true`) or remove (`false`) the pet's hooks in one agent.
    Hooks {
        agent: String,
        install: bool,
    },
}

/// Turn a menu id into an action, but only when it carries `prefix` ("" for the tray).
pub fn parse_action(id: &str, prefix: &str) -> Option<Action> {
    let rest = id.strip_prefix(prefix)?;
    match rest {
        "show" => Some(Action::Show),
        "hide" => Some(Action::Hide),
        "quit" => Some(Action::Quit),
        "map-reset" => Some(Action::MapReset),
        _ => {
            if let Some(wanted) = rest.strip_prefix("play:") {
                return Some(Action::Play(wanted.to_string()));
            }
            if let Some(source) = rest.strip_prefix("src:") {
                return Some(Action::Source(
                    (source != "all").then(|| source.to_string()),
                ));
            }
            if let Some(agent) = rest.strip_prefix("hooks:install:") {
                return Some(Action::Hooks {
                    agent: agent.to_string(),
                    install: true,
                });
            }
            if let Some(agent) = rest.strip_prefix("hooks:remove:") {
                return Some(Action::Hooks {
                    agent: agent.to_string(),
                    install: false,
                });
            }
            // map:<agent>:<event>:<kind>[:<value>]
            let mut parts = rest.strip_prefix("map:")?.splitn(4, ':');
            let (agent, event, kind) = (parts.next()?, parts.next()?, parts.next()?);
            let value = parts.next();
            let (agent, event) = (agent.to_string(), event.to_string());
            match (kind, value) {
                ("default", _) => Some(Action::MapSet {
                    agent,
                    event,
                    choice: None,
                }),
                ("state", Some(v)) => Some(Action::MapSet {
                    agent,
                    event,
                    choice: Some(Choice::State(v.to_string())),
                }),
                ("clip", Some(v)) => Some(Action::MapSet {
                    agent,
                    event,
                    choice: Some(Choice::Clip(v.to_string())),
                }),
                ("off", _) => Some(Action::MapSet {
                    agent,
                    event,
                    choice: Some(Choice::Off(true)),
                }),
                ("last", _) => Some(Action::MapLast { agent, event }),
                _ => None,
            }
        }
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

fn choice_label(choice: Option<&Choice>, clips: &[Clip]) -> String {
    match choice {
        None => "default".to_string(),
        Some(Choice::State(s)) => MAPPABLE_STATES
            .iter()
            .find(|(id, _)| id == s)
            .map_or(s.clone(), |(_, label)| (*label).to_string()),
        Some(Choice::Clip(id)) => clips
            .iter()
            .find(|c| &c.id == id)
            .map_or(id.clone(), |c| c.title.clone()),
        Some(Choice::Off(_)) => "no animation".to_string(),
    }
}

/// One event's submenu: Default, State, Clip (by category), Use last played, No animation.
fn event_submenu(
    app: &AppHandle,
    agent: &str,
    event: &str,
    current: Option<&Choice>,
    clips: &[Clip],
    last: Option<&Clip>,
) -> tauri::Result<Submenu<Wry>> {
    let title = format!("{event}  ({})", choice_label(current, clips));
    let sub = Submenu::with_id(app, format!("mapev:{agent}:{event}"), &title, true)?;
    let id = |kind: &str| format!("map:{agent}:{event}:{kind}");

    sub.append(&CheckMenuItem::with_id(
        app,
        id("default"),
        "Default",
        true,
        current.is_none(),
        None::<&str>,
    )?)?;

    let states = Submenu::with_id(app, format!("mapst:{agent}:{event}"), "State", true)?;
    for (state, label) in MAPPABLE_STATES {
        let checked = matches!(current, Some(Choice::State(s)) if s == state);
        states.append(&CheckMenuItem::with_id(
            app,
            id(&format!("state:{state}")),
            label,
            true,
            checked,
            None::<&str>,
        )?)?;
    }
    sub.append(&states)?;

    let clip_menu = Submenu::with_id(
        app,
        format!("mapcl:{agent}:{event}"),
        "Clip",
        !clips.is_empty(),
    )?;
    for (category, group) in catalog::group_by_category(clips) {
        let cat = Submenu::with_id(
            app,
            format!("mapcat:{agent}:{event}:{category}"),
            &category,
            true,
        )?;
        for clip in group {
            let checked = matches!(current, Some(Choice::Clip(c)) if *c == clip.id);
            cat.append(&CheckMenuItem::with_id(
                app,
                id(&format!("clip:{}", clip.id)),
                &clip.title,
                true,
                checked,
                None::<&str>,
            )?)?;
        }
        clip_menu.append(&cat)?;
    }
    sub.append(&clip_menu)?;

    let last_text = match last {
        Some(c) => format!("Use last played: {}", c.title),
        None => "Use last played clip".to_string(),
    };
    sub.append(&MenuItem::with_id(
        app,
        id("last"),
        &last_text,
        mapping::last_item_enabled(last.map(|c| c.id.as_str())),
        None::<&str>,
    )?)?;
    sub.append(&CheckMenuItem::with_id(
        app,
        id("off"),
        "No animation",
        true,
        matches!(current, Some(Choice::Off(_))),
        None::<&str>,
    )?)?;
    Ok(sub)
}

/// "Map animations": every installed agent, every event it sends, current choice shown in the title.
fn map_menu(
    app: &AppHandle,
    clips: &[Clip],
    mapping: &Mapping,
    last: Option<&Clip>,
    installed: &[&AgentDef],
) -> tauri::Result<Submenu<Wry>> {
    let root = Submenu::with_id(app, "map", "Map animations", true)?;
    if installed.is_empty() {
        root.append(&MenuItem::with_id(
            app,
            "map-none",
            "Install hooks for an agent first",
            false,
            None::<&str>,
        )?)?;
    }
    for def in installed {
        let agent = def.id.as_str();
        let agent_menu = Submenu::with_id(app, format!("mapagent:{agent}"), &def.label, true)?;
        for event in &def.events {
            agent_menu.append(&event_submenu(
                app,
                agent,
                event,
                mapping.get(agent, event),
                clips,
                last,
            )?)?;
        }
        root.append(&agent_menu)?;
    }
    root.append(&PredefinedMenuItem::separator(app)?)?;
    root.append(&MenuItem::with_id(
        app,
        "map-reset",
        "Reset all to defaults",
        true,
        None::<&str>,
    )?)?;
    Ok(root)
}

/// First line of the menu-bar menu, so you can tell which build is running.
pub fn version_label() -> String {
    format!("Big Blue Fish  v{}", env!("CARGO_PKG_VERSION"))
}

/// Title of the "Active source" submenu, naming the current choice.
pub fn source_title(active: Option<&str>) -> String {
    let name = match active {
        None => "all sources".to_string(),
        Some(id) => mapping::agents()
            .iter()
            .find(|a| a.id == id)
            .map_or(id.to_string(), |a| a.label.clone()),
    };
    format!("Active source  ({name})")
}

/// "Active source": which agent drives the pet. Only installed agents (and the current choice) are offered.
fn source_menu(
    app: &AppHandle,
    installed: &[&AgentDef],
    active: Option<&str>,
) -> tauri::Result<Submenu<Wry>> {
    let root = Submenu::with_id(app, "source", source_title(active), true)?;
    root.append(&CheckMenuItem::with_id(
        app,
        "src:all",
        "All sources",
        true,
        active.is_none(),
        None::<&str>,
    )?)?;
    let mut offered: Vec<(String, String)> = installed
        .iter()
        .map(|a| (a.id.clone(), a.label.clone()))
        .collect();
    if let Some(current) = active.filter(|c| !offered.iter().any(|(id, _)| id == c)) {
        let label = mapping::agents()
            .iter()
            .find(|a| a.id == current)
            .map_or(current.to_string(), |a| a.label.clone());
        offered.push((current.to_string(), label));
    }
    for (id, label) in offered {
        root.append(&CheckMenuItem::with_id(
            app,
            format!("src:{id}"),
            &label,
            true,
            active == Some(id.as_str()),
            None::<&str>,
        )?)?;
    }
    Ok(root)
}

/// "Install hooks": one row per agent, click to install or remove.
fn hooks_menu(app: &AppHandle, installer_ok: bool, status: &Status) -> tauri::Result<Submenu<Wry>> {
    let root = Submenu::with_id(app, "hooks", "Install hooks", true)?;
    if !installer_ok {
        root.append(&MenuItem::with_id(
            app,
            "hooks-needs-python",
            "Needs the Xcode command line tools (run: xcode-select --install)",
            false,
            None::<&str>,
        )?)?;
        return Ok(root);
    }
    for row in hooks::install_rows(mapping::agents(), status) {
        let kind = if row.installed { "remove" } else { "install" };
        root.append(&MenuItem::with_id(
            app,
            format!("hooks:{kind}:{}", row.id),
            &row.text,
            row.enabled,
            None::<&str>,
        )?)?;
    }
    Ok(root)
}

/// The menu-bar icon's menu. Rebuilt after every mapping change so the check marks stay right.
pub fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let started = std::time::Instant::now();
    let clips = app.state::<PlayCatalog>().0.clone();
    let store = app.state::<MappingStore>();
    let mapping = store.mapping.lock().map(|m| m.clone()).unwrap_or_default();
    let last_id = store.last_played.lock().ok().and_then(|l| l.clone());
    let last = last_id
        .as_deref()
        .and_then(|id| clips.iter().find(|c| c.id == id));

    let version = MenuItem::with_id(app, "version", version_label(), false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show Big Blue Fish", true, None::<&str>)?;
    let play = Submenu::with_id(app, "play", "Play animation", !clips.is_empty())?;
    let (random, categories) = play_entries(app, &clips, "")?;
    play.append(&random)?;
    let items: Vec<&dyn IsMenuItem<Wry>> = categories
        .iter()
        .map(|c| c as &dyn IsMenuItem<Wry>)
        .collect();
    play.append_items(&items)?;
    let installer = Installer::locate().filter(|i| hooks::python_ready(&i.python));
    let status = installer
        .as_ref()
        .map(Installer::status)
        .unwrap_or_default();
    let installed = hooks::installed_agents(mapping::agents(), &status);
    let active = app
        .try_state::<SettingsStore>()
        .and_then(|s| s.0.lock().ok().and_then(|g| g.active_source.clone()));
    let map = map_menu(app, &clips, &mapping, last, &installed)?;
    let source = source_menu(app, &installed, active.as_deref())?;
    let install = hooks_menu(app, installer.is_some(), &status)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &version,
            &PredefinedMenuItem::separator(app)?,
            &show,
            &play,
            &map,
            &PredefinedMenuItem::separator(app)?,
            &source,
            &install,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    tracing::info!(
        "menu-bar menu built in {} ms",
        started.elapsed().as_millis()
    );
    Ok(menu)
}

fn refresh_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    match build_tray_menu(app) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(e) => tracing::warn!("Cannot rebuild the menu-bar menu: {e}"),
    }
}

fn mapping_path() -> std::path::PathBuf {
    crate::home::user_home_dir().join("mapping.json")
}

fn save_mapping(app: &AppHandle) {
    let store = app.state::<MappingStore>();
    let result = store
        .mapping
        .lock()
        .map(|mapping| mapping.save(&mapping_path()));
    if let Ok(Err(e)) = result {
        tracing::warn!("Cannot save {}: {e}", mapping_path().display());
    }
}

/// Play a clip (id or "random"). `reveal` shows a hidden pet first (menu clicks do, hook events do not).
pub fn play(app: &AppHandle, wanted: &str, reveal: bool) {
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
    let Some(clip) = clip else {
        return;
    };
    if reveal {
        crate::commands::show_pet(app);
    }
    if let Some(store) = app.try_state::<MappingStore>() {
        if let Ok(mut last) = store.last_played.lock() {
            *last = Some(clip.id.clone());
        }
    }
    let _ = app.emit("play-animation", clip);
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
            play(app, &wanted, true);
            refresh_tray(app); // "Use last played" shows the new clip
        }
        Action::MapSet {
            agent,
            event,
            choice,
        } => {
            let preview = match &choice {
                Some(Choice::Clip(id)) => Some(id.clone()),
                _ => None,
            };
            if let Some(store) = app.try_state::<MappingStore>() {
                if let Ok(mut m) = store.mapping.lock() {
                    m.set(&agent, &event, choice);
                }
            }
            save_mapping(app);
            refresh_tray(app);
            if let Some(id) = preview {
                play(app, &id, true);
            }
        }
        Action::MapLast { agent, event } => {
            let choice = app.try_state::<MappingStore>().and_then(|store| {
                let last = store.last_played.lock().ok().and_then(|l| l.clone());
                mapping::assign_last(last.as_deref())
            });
            if choice.is_some() {
                run(
                    app,
                    Action::MapSet {
                        agent,
                        event,
                        choice,
                    },
                );
            }
        }
        Action::Source(source) => {
            if let Some(agg) = app.try_state::<Arc<ActivityAggregator>>() {
                agg.set_active_source(source.clone());
            }
            if let Some(store) = app.try_state::<SettingsStore>() {
                if let Ok(mut settings) = store.0.lock() {
                    settings.active_source = source;
                    if let Err(e) =
                        settings.save(&crate::home::user_home_dir().join("settings.json"))
                    {
                        tracing::warn!("Cannot save settings: {e}");
                    }
                }
            }
            refresh_tray(app);
        }
        Action::Hooks { agent, install } => {
            // blocking and short (it rewrites one small config file); the menu is rebuilt after
            match Installer::locate() {
                Some(installer) => {
                    if let Err(e) = installer.set_installed(&agent, install) {
                        tracing::warn!(
                            "Cannot {} hooks for {agent}: {e}",
                            if install { "install" } else { "remove" }
                        );
                    }
                }
                None => tracing::warn!("Hook installer not found"),
            }
            refresh_tray(app);
        }
        Action::MapReset => {
            if let Some(store) = app.try_state::<MappingStore>() {
                if let Ok(mut m) = store.mapping.lock() {
                    m.reset();
                }
            }
            save_mapping(app);
            refresh_tray(app);
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
    fn ac_0_7_4_map_menu_ids_parse() {
        assert_eq!(
            parse_action("map:claude-code:Stop:clip:c042", ""),
            Some(Action::MapSet {
                agent: "claude-code".into(),
                event: "Stop".into(),
                choice: Some(Choice::Clip("c042".into()))
            })
        );
        assert_eq!(
            parse_action("map:zai:PostToolUseFailure:state:failed", ""),
            Some(Action::MapSet {
                agent: "zai".into(),
                event: "PostToolUseFailure".into(),
                choice: Some(Choice::State("failed".into()))
            })
        );
        assert_eq!(
            parse_action("map:zai:Stop:default", ""),
            Some(Action::MapSet {
                agent: "zai".into(),
                event: "Stop".into(),
                choice: None
            })
        );
        assert_eq!(
            parse_action("map:zai:Stop:last", ""),
            Some(Action::MapLast {
                agent: "zai".into(),
                event: "Stop".into()
            })
        );
        assert_eq!(parse_action("map-reset", ""), Some(Action::MapReset));
        assert_eq!(parse_action("map:zai:Stop:bogus", ""), None);
    }

    #[test]
    fn ac_0_7_4_agent_event_tables_match_installer() {
        // the menu and the installer read one file, so the 0.7.0 agents keep their events
        let claude = mapping::agents()
            .iter()
            .find(|a| a.id == "claude-code")
            .unwrap();
        for event in ["Stop", "PreToolUse", "PermissionRequest", "SessionEnd"] {
            assert!(claude.events.iter().any(|e| e == event), "{event}");
        }
        let zai = mapping::agents().iter().find(|a| a.id == "zai").unwrap();
        assert!(zai.events.iter().any(|e| e == "PostToolUseFailure"));
        let table = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hooks/agents.json"),
        )
        .unwrap();
        let ids: Vec<&str> = mapping::agents().iter().map(|a| a.id.as_str()).collect();
        for id in ids {
            assert!(table.contains(&format!("\"id\": \"{id}\"")));
        }
    }

    #[test]
    fn ac_0_8_9_menu_shows_version() {
        let label = version_label();
        assert!(label.ends_with(env!("CARGO_PKG_VERSION")), "{label}");
        assert!(label.contains(" v"));
    }

    #[test]
    fn ac_0_8_9_source_title_names_current_choice() {
        assert_eq!(source_title(None), "Active source  (all sources)");
        assert_eq!(
            source_title(Some("claude-code")),
            "Active source  (Claude Code)"
        );
        assert_eq!(source_title(Some("zai")), "Active source  (Z.ai ZCode)");
    }

    #[test]
    fn ac_0_8_7_source_and_hook_menu_ids_parse() {
        assert_eq!(parse_action("src:all", ""), Some(Action::Source(None)));
        assert_eq!(
            parse_action("src:claude-code", ""),
            Some(Action::Source(Some("claude-code".into())))
        );
        assert_eq!(
            parse_action("hooks:install:gemini", ""),
            Some(Action::Hooks {
                agent: "gemini".into(),
                install: true
            })
        );
        assert_eq!(
            parse_action("hooks:remove:gemini", ""),
            Some(Action::Hooks {
                agent: "gemini".into(),
                install: false
            })
        );
        // the popup never handles these
        assert_eq!(parse_action("src:all", POPUP_PREFIX), None);
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
