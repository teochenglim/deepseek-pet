//! "Show Claude context" floats in a small window of its own, next to the pet. It shares nothing
//! with the pet's page or window: no speech bubble, no resizing of the pet window, no state. It
//! opens, shows the text, fades over its last two seconds and closes itself after `NOTE_MS`
//! (click it to close sooner). Opening it again replaces it. Whatever happens to it, the pet is
//! not affected (0.9.4 to 0.9.6 drew the note in the pet's bubble and grew its window, which
//! could push the pet out of view).
//!
//! Also here: the rescue behind "Show Big Blue Fish" (the pet's page is reloaded so it always
//! draws; the window keeps its place if it is on a screen, else the page puts it in its corner).
use crate::context::NOTE_MS;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    AppHandle, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// x, y, width, height in physical pixels.
pub type Rect = (i32, i32, u32, u32);

const WIDTH: f64 = 340.0;
const CHARS_PER_LINE: usize = 46;
const LINE_HEIGHT: f64 = 17.0;
const PADDING: f64 = 26.0;
const GAP: i32 = 6;
const LABEL_PREFIX: &str = "context-";

// ── geometry (pure, tested) ─────────────────────────────────────────────────────────────────

/// Logical size of the note window for `text`: fixed width, as tall as its wrapped lines.
pub fn context_size(text: &str) -> (f64, f64) {
    let lines: usize = text
        .lines()
        .map(|l| l.chars().count().div_ceil(CHARS_PER_LINE).max(1))
        .sum::<usize>()
        .max(1);
    (WIDTH, lines as f64 * LINE_HEIGHT + PADDING)
}

/// Where the note goes: above the pet window, right edges aligned, `GAP` clear of it; below the pet
/// when there is no room above; always inside `monitor`.
pub fn context_position(pet: Rect, size: (u32, u32), monitor: Rect) -> (i32, i32) {
    let (w, h) = (size.0 as i32, size.1 as i32);
    let mut x = pet.0 + pet.2 as i32 - w;
    let mut y = pet.1 - h - GAP;
    if y < monitor.1 {
        y = pet.1 + pet.3 as i32 + GAP;
    }
    x = x.clamp(monitor.0, (monitor.0 + monitor.2 as i32 - w).max(monitor.0));
    y = y.clamp(monitor.1, (monitor.1 + monitor.3 as i32 - h).max(monitor.1));
    (x, y)
}

/// Whether the window's centre is on any screen. A pet whose window is on no screen cannot be seen.
pub fn center_visible(monitors: &[Rect], win: Rect) -> bool {
    let cx = i64::from(win.0) + i64::from(win.2) / 2;
    let cy = i64::from(win.1) + i64::from(win.3) / 2;
    monitors.iter().any(|m| {
        cx >= i64::from(m.0)
            && cx < i64::from(m.0) + i64::from(m.2)
            && cy >= i64::from(m.1)
            && cy < i64::from(m.1) + i64::from(m.3)
    })
}

/// The position worth keeping across a reload: where the window is, if it is on a screen.
pub fn keep_position(monitors: &[Rect], win: Rect) -> Option<(i32, i32)> {
    center_visible(monitors, win).then_some((win.0, win.1))
}

/// Script run in the note window before its page: hands the page the text and how long it stays.
pub fn init_script(text: &str) -> String {
    format!(
        "window.__NOTE = {};",
        serde_json::json!({"text": text, "ms": NOTE_MS})
    )
}

// ── the floating window ─────────────────────────────────────────────────────────────────────

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn rect_of(win: &WebviewWindow) -> Option<Rect> {
    let p = win.outer_position().ok()?;
    let s = win.outer_size().ok()?;
    Some((p.x, p.y, s.width, s.height))
}

fn monitors_of(win: &WebviewWindow) -> Vec<Rect> {
    win.available_monitors()
        .map(|list| {
            list.iter()
                .map(|m| {
                    (
                        m.position().x,
                        m.position().y,
                        m.size().width,
                        m.size().height,
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Open the note next to the pet (or at a sensible place if the pet window cannot be measured).
pub fn open(app: &AppHandle, text: &str) -> tauri::Result<()> {
    for (label, win) in app.webview_windows() {
        if label.starts_with(LABEL_PREFIX) {
            let _ = win.close(); // opening it again replaces it
        }
    }
    let label = format!("{LABEL_PREFIX}{}", COUNTER.fetch_add(1, Ordering::Relaxed));
    let (w, h) = context_size(text);
    let note = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("context.html".into()))
        .title("Claude context")
        .inner_size(w, h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .initialization_script(init_script(text))
        .build()?;
    if let (Some(pet), Ok(size)) = (app.get_webview_window("main"), note.outer_size()) {
        if let Some(pet_rect) = rect_of(&pet) {
            let monitors = monitors_of(&pet);
            let monitor = monitors
                .iter()
                .copied()
                .find(|m| center_visible(&[*m], pet_rect))
                .or_else(|| monitors.first().copied());
            if let Some(monitor) = monitor {
                let (x, y) = context_position(pet_rect, (size.width, size.height), monitor);
                let _ = note.set_position(PhysicalPosition::new(x, y));
            }
        }
    }
    note.show()?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(NOTE_MS + 500)).await;
        if let Some(win) = handle.get_webview_window(&label) {
            let _ = win.close();
        }
    });
    Ok(())
}

// ── Show Big Blue Fish ──────────────────────────────────────────────────────────────────────

/// Where the pet window was before "Show Big Blue Fish" reloaded its page; the page asks for it
/// (once) when it has laid itself out again.
#[derive(Default)]
pub struct RestorePos(pub Mutex<Option<(i32, i32)>>);

/// Remember the pet window's place, if it is on a screen (otherwise the reloaded page puts it in
/// its default corner, which is always on the primary screen).
pub fn remember_position(app: &AppHandle) {
    let (Some(win), Some(state)) = (
        app.get_webview_window("main"),
        app.try_state::<RestorePos>(),
    ) else {
        return;
    };
    let keep = rect_of(&win).and_then(|r| keep_position(&monitors_of(&win), r));
    let mut slot = match state.0.lock() {
        Ok(slot) => slot,
        Err(_) => return,
    };
    *slot = keep;
}

/// Called by the page after it positioned itself: move the window back to where it was.
#[tauri::command]
pub fn apply_restore_position(app: AppHandle, state: State<RestorePos>) -> Result<(), String> {
    let saved = state.0.lock().map_err(|e| e.to_string())?.take();
    if let (Some((x, y)), Some(win)) = (saved, app.get_webview_window("main")) {
        win.set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = (0, 0, 2560, 1440);

    #[test]
    fn ac_0_9_11_a_window_off_every_screen_is_detected() {
        let screens = [(0, 0, 2560, 1440), (2560, 0, 1920, 1080)];
        assert!(center_visible(&screens, (2000, 1000, 280, 440)));
        assert!(center_visible(&screens, (2600, 100, 280, 440)));
        assert!(!center_visible(&screens, (9000, 100, 280, 440)));
        assert!(!center_visible(&screens, (-3000, -3000, 280, 440)));
        assert!(!center_visible(&[], (0, 0, 280, 440)));
        // only its edge on a screen is not enough: the pet is drawn in the middle
        assert!(!center_visible(&screens, (-270, 100, 280, 440)));
    }

    #[test]
    fn ac_0_9_11_the_windows_centre_decides_not_its_edge() {
        // straddling two side-by-side screens: the centre is on the second one
        let screens = [(0, 0, 1000, 800), (1000, 0, 1000, 800)];
        assert!(center_visible(&screens, (900, 100, 280, 440)));
        // hanging off the bottom of a screen (bottom edge at 940) with the centre (y 720) inside
        assert!(center_visible(&screens, (100, 500, 280, 440)));
        // the centre exactly on the bottom edge (y 800) is off every screen
        assert!(!center_visible(&screens, (100, 580, 280, 440)));
    }

    #[test]
    fn ac_0_9_12_the_page_gets_the_text_and_the_duration() {
        let script = init_script("Claude context\n42% \"quoted\"");
        assert!(script.starts_with("window.__NOTE = {"), "{script}");
        assert!(script.contains("\"ms\":20000"), "{script}");
        assert!(script.contains("42%"), "{script}");
        // the text is a JSON string, so a quote or newline in it cannot break out of the script
        assert!(
            script.contains("\\n") && script.contains("\\\"quoted\\\""),
            "{script}"
        );
    }

    #[test]
    fn ac_0_9_13_the_note_sits_above_the_pet_aligned_right() {
        // pet window 280 x 440 at the bottom-right; note 680 x 120 (physical)
        let pet = (2240, 960, 280, 440);
        let (x, y) = context_position(pet, (680, 120), SCREEN);
        assert_eq!(x + 680, pet.0 + 280, "right edges line up");
        assert_eq!(y + 120 + GAP, pet.1, "sits just above the pet window");
    }

    #[test]
    fn ac_0_9_13_the_note_goes_below_when_there_is_no_room_above_and_stays_on_screen() {
        // pet at the top edge: no room above
        let pet = (1000, 10, 280, 440);
        let (_, y) = context_position(pet, (680, 120), SCREEN);
        assert_eq!(y, pet.1 + 440 + GAP);
        // pet at the far left: the note is pulled inside the screen
        let (x, _) = context_position((0, 900, 280, 440), (680, 120), SCREEN);
        assert_eq!(x, 0);
        // never outside the monitor, even when it does not fit anywhere
        let (x, y) = context_position((5000, 5000, 280, 440), (680, 120), SCREEN);
        assert!(x + 680 <= 2560 && y + 120 <= 1440 && x >= 0 && y >= 0);
    }

    #[test]
    fn ac_0_9_13_the_note_size_follows_the_text() {
        let (w, short) = context_size("one line");
        assert_eq!(w, WIDTH);
        let (_, four) = context_size("a\nb\nc\nd");
        assert!(four > short);
        // a long line wraps into more than one row
        let (_, wrapped) = context_size(&"x".repeat(CHARS_PER_LINE * 2 + 1));
        assert!(wrapped > short);
        assert!(context_size("").1 > 0.0);
    }

    #[test]
    fn ac_0_9_14_the_pets_place_is_kept_across_the_reload_when_it_is_on_a_screen() {
        assert_eq!(
            keep_position(&[SCREEN], (2240, 960, 280, 440)),
            Some((2240, 960))
        );
        let state = RestorePos::default();
        *state.0.lock().unwrap() = Some((10, 20));
        assert_eq!(state.0.lock().unwrap().take(), Some((10, 20)));
        assert_eq!(state.0.lock().unwrap().take(), None, "applied only once");
    }

    #[test]
    fn ac_0_9_14_an_off_screen_pet_is_not_kept_so_the_reload_brings_it_back() {
        assert_eq!(keep_position(&[SCREEN], (9000, 9000, 280, 440)), None);
        assert_eq!(keep_position(&[], (0, 0, 280, 440)), None);
    }
}
