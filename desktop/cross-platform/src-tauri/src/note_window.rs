//! The pet window is only a little larger than the pet, so a multi-line speech bubble (Show Claude
//! context) would push the pet out of view. While a note is showing the window grows to the left and
//! upward, and shrinks back afterwards, so the pet itself never moves on screen.
//!
//! All sizes here are logical pixels. `grow` and `shrink` are pure so they can be tested.
use std::sync::Mutex;
use tauri::{LogicalPosition, LogicalSize, Manager, State};

type Pair = (f64, f64);

/// The growth currently applied to the window (width, height), if any.
#[derive(Default)]
pub struct NoteWindow(pub Mutex<Option<Pair>>);

/// New (position, size, growth) for a window at `pos` with `size` that must be `target_w` wide and
/// `extra_h` taller. The pet stays centred horizontally and keeps its bottom edge in place.
pub fn grow(pos: Pair, size: Pair, target_w: f64, extra_h: f64) -> (Pair, Pair, Pair) {
    let dw = (target_w - size.0).max(0.0);
    let dh = extra_h.max(0.0);
    (
        (pos.0 - dw / 2.0, pos.1 - dh),
        (size.0 + dw, size.1 + dh),
        (dw, dh),
    )
}

/// Undo a `grow` (the window may have been dragged meanwhile, so this works from where it is now).
pub fn shrink(pos: Pair, size: Pair, growth: Pair) -> (Pair, Pair) {
    (
        (pos.0 + growth.0 / 2.0, pos.1 + growth.1),
        (size.0 - growth.0, size.1 - growth.1),
    )
}

fn geometry(app: &tauri::AppHandle) -> Result<(tauri::WebviewWindow, Pair, Pair), String> {
    let win = app.get_webview_window("main").ok_or("no main window")?;
    let sf = win.scale_factor().map_err(|e| e.to_string())?;
    let p = win
        .outer_position()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(sf);
    let s = win
        .outer_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(sf);
    Ok((win, (p.x, p.y), (s.width, s.height)))
}

fn apply(win: &tauri::WebviewWindow, pos: Pair, size: Pair) -> Result<(), String> {
    win.set_size(LogicalSize::new(size.0, size.1))
        .map_err(|e| e.to_string())?;
    win.set_position(LogicalPosition::new(pos.0, pos.1))
        .map_err(|e| e.to_string())
}

/// Make room for a note: `width` wide and `extra_height` taller than the pet window. Calling it
/// again re-measures from the original size.
#[tauri::command]
pub fn note_window_grow(
    app: tauri::AppHandle,
    state: State<NoteWindow>,
    width: f64,
    extra_height: f64,
) -> Result<(), String> {
    let (win, mut pos, mut size) = geometry(&app)?;
    let mut current = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(applied) = current.take() {
        (pos, size) = shrink(pos, size, applied);
    }
    let (pos, size, growth) = grow(pos, size, width, extra_height);
    apply(&win, pos, size)?;
    *current = Some(growth);
    Ok(())
}

/// The note is gone: give the window back its pet-only size.
#[tauri::command]
pub fn note_window_restore(app: tauri::AppHandle, state: State<NoteWindow>) -> Result<(), String> {
    let mut current = state.0.lock().map_err(|e| e.to_string())?;
    let Some(applied) = current.take() else {
        return Ok(());
    };
    let (win, pos, size) = geometry(&app)?;
    let (pos, size) = shrink(pos, size, applied);
    apply(&win, pos, size)
}

/// x, y, width, height in physical pixels.
pub type Rect = (i32, i32, u32, u32);

/// Whether the window's centre is on any screen. A pet whose window ended up off every screen
/// (a monitor unplugged, a bad drag) cannot be seen or reached.
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

/// Bottom-right corner of `monitor` for a window of `win`'s size, `margin` pixels from the edges
/// (where the pet starts).
pub fn corner_position(monitor: Rect, win: Rect, margin: i32) -> (i32, i32) {
    (
        monitor.0 + monitor.2 as i32 - win.2 as i32 - margin,
        monitor.1 + monitor.3 as i32 - win.3 as i32 - margin,
    )
}

/// "Show Big Blue Fish": whatever a note did to the window is undone, and a window that is off
/// every screen goes back to the primary screen's corner. Errors are ignored: this is a rescue.
pub fn reset(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<NoteWindow>() {
        let _ = note_window_restore(app.clone(), state);
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let (Ok(pos), Ok(size), Ok(monitors)) = (
        win.outer_position(),
        win.outer_size(),
        win.available_monitors(),
    ) else {
        return;
    };
    let rects: Vec<Rect> = monitors
        .iter()
        .map(|m| {
            (
                m.position().x,
                m.position().y,
                m.size().width,
                m.size().height,
            )
        })
        .collect();
    let rect = (pos.x, pos.y, size.width, size.height);
    if rects.is_empty() || center_visible(&rects, rect) {
        return;
    }
    let primary = win
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            (
                m.position().x,
                m.position().y,
                m.size().width,
                m.size().height,
            )
        })
        .unwrap_or(rects[0]);
    let margin = (20.0 * win.scale_factor().unwrap_or(1.0)) as i32;
    let (x, y) = corner_position(primary, rect, margin);
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_0_9_9_note_window_grows_up_and_sideways_so_the_pet_stays_put() {
        // pet window 140 x 220 at (1000, 600); the note wants 300 wide and 80 taller
        let (pos, size, growth) = grow((1000.0, 600.0), (140.0, 220.0), 300.0, 80.0);
        assert_eq!(size, (300.0, 300.0));
        assert_eq!(growth, (160.0, 80.0));
        // centre x and bottom edge are unchanged: that is where the pet is drawn
        assert_eq!(pos.0 + size.0 / 2.0, 1000.0 + 140.0 / 2.0);
        assert_eq!(pos.1 + size.1, 600.0 + 220.0);
        // a window that is already wide enough only grows taller
        let (_, size, growth) = grow((0.0, 100.0), (400.0, 200.0), 300.0, 50.0);
        assert_eq!((size, growth), ((400.0, 250.0), (0.0, 50.0)));
        // negative or zero extra never shrinks it
        assert_eq!(grow((0.0, 0.0), (10.0, 10.0), 10.0, -5.0).1, (10.0, 10.0));
    }

    #[test]
    fn ac_0_9_9_restore_returns_the_original_geometry_even_after_a_drag() {
        let (pos, size, growth) = grow((1000.0, 600.0), (140.0, 220.0), 300.0, 80.0);
        assert_eq!(shrink(pos, size, growth), ((1000.0, 600.0), (140.0, 220.0)));
        // dragged 50 right and 30 down while the note was up: the pet window lands 50 / 30 away
        let dragged = (pos.0 + 50.0, pos.1 + 30.0);
        assert_eq!(
            shrink(dragged, size, growth),
            ((1050.0, 630.0), (140.0, 220.0))
        );
    }

    #[test]
    fn ac_0_9_11_a_window_off_every_screen_is_detected() {
        let screens = [(0, 0, 2560, 1440), (2560, 0, 1920, 1080)];
        assert!(center_visible(&screens, (2000, 1000, 280, 440)));
        assert!(center_visible(&screens, (2600, 100, 280, 440)));
        // dragged far away, or its monitor was unplugged
        assert!(!center_visible(&screens, (9000, 100, 280, 440)));
        assert!(!center_visible(&screens, (-3000, -3000, 280, 440)));
        assert!(!center_visible(&[], (0, 0, 280, 440)));
        // only its edge on a screen is not enough: the pet is drawn in the middle
        assert!(!center_visible(&screens, (-270, 100, 280, 440)));
    }

    #[test]
    fn ac_0_9_11_an_off_screen_window_goes_to_the_primary_screens_corner() {
        let primary = (0, 0, 2560, 1440);
        let (x, y) = corner_position(primary, (9000, 9000, 280, 440), 40);
        assert_eq!((x, y), (2560 - 280 - 40, 1440 - 440 - 40));
        assert!(center_visible(&[primary], (x, y, 280, 440)));
        // a primary monitor that does not start at the origin
        assert_eq!(
            corner_position((1920, 0, 1920, 1080), (0, 0, 280, 440), 40),
            (1920 + 1920 - 280 - 40, 1080 - 440 - 40)
        );
    }
}
