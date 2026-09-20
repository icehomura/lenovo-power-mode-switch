use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

/// Runtime state of the floater window, shared across commands.
pub struct FloaterState {
    pub visible: Mutex<bool>,
    pub style: Mutex<FloaterStyle>,
    /// Screen corner the floater is anchored to.
    pub position: Mutex<FloaterPosition>,
}

/// Screen corner the floater snaps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FloaterPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl FloaterPosition {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "top-left" => Some(Self::TopLeft),
            "top-right" => Some(Self::TopRight),
            "bottom-left" => Some(Self::BottomLeft),
            "bottom-right" => Some(Self::BottomRight),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
        }
    }
}

/// Gap between the floater and the screen edge, in logical pixels.
const EDGE_MARGIN: f64 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FloaterStyle {
    Capsule,
    Circle,
}

impl FloaterStyle {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "capsule" => Some(Self::Capsule),
            "circle" => Some(Self::Circle),
            _ => None,
        }
    }
}

/// Serialized status returned to the frontend.
#[derive(Serialize)]
pub struct FloaterStatus {
    pub visible: bool,
    pub style: &'static str,
    pub position: &'static str,
}

impl FloaterState {
    pub fn new() -> Self {
        Self {
            visible: Mutex::new(false),
            style: Mutex::new(FloaterStyle::Capsule),
            position: Mutex::new(FloaterPosition::TopRight),
        }
    }
}

pub const FLOATER_LABEL: &str = "floater";

/// Logical window size for each style.
///
/// The capsule is the wide one: it fits the mode icon *and* its name, so it
/// reads as a label. The circle is the compact one and shows the icon only.
///
/// Capsule width budget (mirrored by `--floater-capsule-w` in styles.css):
///   content = icon 15 + gap 7 + name 2 full-width chars @11px (22) = 44
///   width   = 72, leaving 28 of slack that `justify-content: center` splits
///             evenly into 14px of breathing room on each side
///
/// The slack is deliberate headroom: a longer name consumes it symmetrically
/// before the label starts to ellipsis.
fn style_size(style: FloaterStyle) -> (f64, f64) {
    match style {
        FloaterStyle::Capsule => (72.0, 36.0),
        FloaterStyle::Circle => (36.0, 36.0),
    }
}

/// Return the floater window, creating it on first use.
///
/// The window is built at runtime instead of being declared in
/// `tauri.conf.json` so that it only exists while the user wants it — which is
/// what makes "新建悬浮窗" a real action rather than a no-op.
///
/// The `bool` reports whether this call created the window. That is the only
/// moment the configured corner is applied, which is what makes the setting an
/// *initial* position rather than one that overrides the user's drags.
fn ensure_floater(app: &AppHandle) -> Result<(WebviewWindow, bool), String> {
    if let Some(win) = app.get_webview_window(FLOATER_LABEL) {
        return Ok((win, false));
    }

    let style = {
        let state = app.state::<FloaterState>();
        let current = *state.style.lock().map_err(|e| e.to_string())?;
        current
    };
    let (w, h) = style_size(style);

    let win = WebviewWindowBuilder::new(
        app,
        FLOATER_LABEL,
        WebviewUrl::App("index.html?window=floater".into()),
    )
    .title("")
    .inner_size(w, h)
    .decorations(false)
    .transparent(true)
    // `shadow` defaults to true, which on Windows makes an undecorated window
    // draw a 1px white border (plus rounded corners on Win11). With a
    // transparent background that border shows up as a stray outline around the
    // capsule/circle, so it has to be turned off explicitly.
    .shadow(false)
    .always_on_top(true)
    .resizable(false)
    .skip_taskbar(true)
    .visible(false)
    .focused(false)
    .build()
    .map_err(|e| format!("创建悬浮窗失败: {}", e))?;

    // A close request (Alt+F4) should hide rather than tear the window down, so
    // the floater can come back without being rebuilt from scratch.
    let handle = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Some(w) = handle.get_webview_window(FLOATER_LABEL) {
                let _ = w.hide();
            }
            if let Ok(mut vis) = handle.state::<FloaterState>().visible.lock() {
                *vis = false;
            }
        }
    });

    Ok((win, true))
}

/// Apply a style's size to a live floater window.
fn resize_to_style(win: &WebviewWindow, style: FloaterStyle) {
    let (w, h) = style_size(style);
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
}

/// Snap the floater window to a screen corner of whichever monitor it is on.
///
/// Uses the monitor's own origin as the reference so multi-monitor setups
/// anchor to the correct screen rather than always to the primary one.
fn apply_position(win: &tauri::WebviewWindow, position: FloaterPosition) -> Result<(), String> {
    let monitor = win
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| win.primary_monitor().ok().flatten())
        .ok_or("no monitor available")?;

    let screen = monitor.size();
    let origin = monitor.position();
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let margin = (EDGE_MARGIN * monitor.scale_factor()) as i32;

    let right = screen.width as i32 - size.width as i32 - margin;
    let bottom = screen.height as i32 - size.height as i32 - margin;

    let (x, y) = match position {
        FloaterPosition::TopLeft => (margin, margin),
        FloaterPosition::TopRight => (right, margin),
        FloaterPosition::BottomLeft => (margin, bottom),
        FloaterPosition::BottomRight => (right, bottom),
    };

    win.set_position(tauri::PhysicalPosition::new(
        origin.x + x.max(0),
        origin.y + y.max(0),
    ))
    .map_err(|e| e.to_string())
}

/// Emit a style-change event so the floater webview can react.
fn notify_style(app: &AppHandle, style: FloaterStyle) {
    let name = match style {
        FloaterStyle::Capsule => "capsule",
        FloaterStyle::Circle => "circle",
    };
    let _ = app.emit("floater-style-changed", name);
}

// ---------------------------------------------------------------------------
// Tauri commands
//
// Every command below is `async` on purpose, and that is load-bearing.
//
// On Windows, `WebviewWindowBuilder::build` and the window methods that marshal
// onto the main thread DEADLOCK when called from a synchronous command — Tauri
// documents this as a WebView2 known issue ("You should use `async` commands and
// separate threads when creating windows"). A synchronous command runs ON the
// main thread and then blocks that same thread waiting for the window
// operation, so it can never complete. The app freezes and every later IPC call
// hangs behind it, which looks exactly like "the toggle does nothing".
//
// An `async` command runs on the async runtime instead, leaving the main thread
// free to service the window operation. `restore_from_settings` has the same
// constraint and therefore spawns a thread.
// ---------------------------------------------------------------------------

/// Sync core of `show_floater`.
///
/// Must NOT be called from the main thread — see the note above.
fn show_floater_impl(app: &AppHandle) -> Result<(), String> {
    let (win, created) =
        ensure_floater(app).inspect_err(|e| eprintln!("[floater] 创建失败: {}", e))?;

    // The style always wins, since a shape change needs the matching window
    // size regardless of where the floater currently sits.
    let (position, style) = {
        let state = app.state::<FloaterState>();
        let p = *state.position.lock().map_err(|e| e.to_string())?;
        let s = *state.style.lock().map_err(|e| e.to_string())?;
        (p, s)
    };
    resize_to_style(&win, style);

    // The configured corner is only the *initial* position. Re-applying it on
    // every show would silently undo wherever the user had dragged the floater.
    if created {
        apply_position(&win, position).inspect_err(|e| eprintln!("[floater] 定位失败: {}", e))?;
    }

    win.show()
        .map_err(|e| format!("显示失败: {}", e))
        .inspect_err(|e| eprintln!("[floater] {}", e))?;
    // A floater that cannot take focus is still usable; do not fail the whole
    // command (and leave `visible` stale) just because the OS refused focus.
    if let Err(e) = win.set_focus() {
        eprintln!("[floater] 获取焦点失败（已忽略）: {}", e);
    }

    let state = app.state::<FloaterState>();
    {
        let mut guard = state.visible.lock().map_err(|e| e.to_string())?;
        *guard = true;
    }
    Ok(())
}

/// Sync core of `hide_floater`. Must NOT be called from the main thread.
fn hide_floater_impl(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(FLOATER_LABEL) {
        win.hide().map_err(|e| e.to_string())?;
    }

    let state = app.state::<FloaterState>();
    *state.visible.lock().map_err(|e| e.to_string())? = false;
    Ok(())
}

#[tauri::command]
pub async fn show_floater(app: AppHandle) -> Result<(), String> {
    show_floater_impl(&app)
}

/// Hide the floater, keeping the window alive for a fast re-show.
#[tauri::command]
pub async fn hide_floater(app: AppHandle) -> Result<(), String> {
    hide_floater_impl(&app)
}

/// Tear the floater window down entirely.
///
/// Used when the user turns the feature off, so that turning it back on really
/// does create a fresh window.
#[tauri::command]
pub async fn destroy_floater(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(FLOATER_LABEL) {
        // `destroy` skips the close-request hook that normally only hides.
        win.destroy().map_err(|e| e.to_string())?;
    }

    let state = app.state::<FloaterState>();
    *state.visible.lock().map_err(|e| e.to_string())? = false;
    Ok(())
}

/// Sync core of `toggle_floater`. Must NOT be called from the main thread.
pub fn toggle_floater_sync(app: &AppHandle) -> Result<bool, String> {
    // Derived from the live window rather than the cached flag, so the answer
    // stays correct even if the window was closed from somewhere else.
    let is_visible = app
        .get_webview_window(FLOATER_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if is_visible {
        hide_floater_impl(app)?;
        Ok(false)
    } else {
        show_floater_impl(app)?;
        Ok(true)
    }
}

#[tauri::command]
pub async fn toggle_floater(app: AppHandle) -> Result<bool, String> {
    toggle_floater_sync(&app)
}

#[tauri::command]
pub async fn set_floater_style(app: AppHandle, style: String) -> Result<(), String> {
    let new_style =
        FloaterStyle::from_str(&style).ok_or_else(|| format!("unknown style: {}", style))?;

    let position = {
        let state = app.state::<FloaterState>();
        let current = *state.position.lock().map_err(|e| e.to_string())?;
        *state.style.lock().map_err(|e| e.to_string())? = new_style;
        current
    };

    // Resize and re-anchor a live window so the new shape stays on screen.
    if let Some(win) = app.get_webview_window(FLOATER_LABEL) {
        resize_to_style(&win, new_style);
        let _ = apply_position(&win, position);
    }

    notify_style(&app, new_style);
    Ok(())
}

#[tauri::command]
pub async fn set_floater_position(app: AppHandle, position: String) -> Result<(), String> {
    let new_position = FloaterPosition::parse(&position)
        .ok_or_else(|| format!("unknown position: {}", position))?;

    {
        let state = app.state::<FloaterState>();
        *state.position.lock().map_err(|e| e.to_string())? = new_position;
    }

    // Move it now if the window exists; it is fine to only record the corner
    // otherwise, since `show_floater` re-applies it.
    if let Some(win) = app.get_webview_window(FLOATER_LABEL) {
        apply_position(&win, new_position)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_floater_status(app: AppHandle) -> Result<FloaterStatus, String> {
    let state = app.state::<FloaterState>();
    // Prefer what the window itself reports; the cached flag is only a
    // fallback for when no window has been created yet.
    let visible = app
        .get_webview_window(FLOATER_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    let style = *state.style.lock().map_err(|e| e.to_string())?;
    let position = *state.position.lock().map_err(|e| e.to_string())?;
    Ok(FloaterStatus {
        visible,
        style: match style {
            FloaterStyle::Capsule => "capsule",
            FloaterStyle::Circle => "circle",
        },
        position: position.as_str(),
    })
}

/// Re-apply the persisted floater settings at startup.
///
/// Reads `settings.json` once so the floater comes back with the style, corner
/// and visibility the user left it in, without the frontend being involved.
pub fn restore_from_settings(app: &AppHandle) {
    use tauri_plugin_store::StoreExt;

    let floating = app
        .store("settings.json")
        .ok()
        .and_then(|s| s.get("settings"))
        .and_then(|v| v.get("floating").cloned());

    let Some(floating) = floating else {
        return;
    };

    {
        let state = app.state::<FloaterState>();
        if let Some(style) = floating.get("style").and_then(|v| v.as_str()) {
            if let Some(parsed) = FloaterStyle::from_str(style) {
                if let Ok(mut guard) = state.style.lock() {
                    *guard = parsed;
                }
            }
        }
        if let Some(pos) = floating.get("position").and_then(|v| v.as_str()) {
            if let Some(parsed) = FloaterPosition::parse(pos) {
                if let Ok(mut guard) = state.position.lock() {
                    *guard = parsed;
                }
            }
        }
    }

    let enabled = floating
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if enabled {
        // Spawned, not called inline: this runs from `Builder::setup` on the
        // main thread, and creating a window there deadlocks (see the note on
        // the commands above). The short delay lets the event loop come up so
        // the window lands on a running loop rather than a booting one.
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if let Err(e) = show_floater_impl(&app) {
                eprintln!("[floater] 启动恢复悬浮窗失败: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floater_state_default_values() {
        let state = FloaterState::new();
        assert!(!*state.visible.lock().unwrap());
        assert_eq!(*state.style.lock().unwrap(), FloaterStyle::Capsule);
        assert_eq!(*state.position.lock().unwrap(), FloaterPosition::TopRight);
    }

    #[test]
    fn floater_state_mutex_thread_safety() {
        let state = FloaterState::new();
        let state = std::sync::Arc::new(state);
        let mut handles = vec![];

        for _ in 0..10 {
            let s = state.clone();
            handles.push(std::thread::spawn(move || {
                let mut vis = s.visible.lock().unwrap();
                *vis = true;
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert!(*state.visible.lock().unwrap());
    }

    #[test]
    fn floater_style_from_str_capsule() {
        assert_eq!(FloaterStyle::from_str("capsule"), Some(FloaterStyle::Capsule));
    }

    // ========================================================================
    // style_size
    // ========================================================================

    #[test]
    fn capsule_is_the_wider_style() {
        let (cw, _) = style_size(FloaterStyle::Capsule);
        let (rw, _) = style_size(FloaterStyle::Circle);
        // The capsule carries a label, so it has to be wider than the circle.
        assert!(cw > rw, "capsule ({}) must be wider than circle ({})", cw, rw);
    }

    #[test]
    fn circle_is_square_so_it_stays_round() {
        let (w, h) = style_size(FloaterStyle::Circle);
        assert_eq!(w, h, "a non-square box would render as an ellipse");
    }

    #[test]
    fn style_sizes_match_the_documented_values() {
        assert_eq!(style_size(FloaterStyle::Capsule), (72.0, 36.0));
        assert_eq!(style_size(FloaterStyle::Circle), (36.0, 36.0));
    }

    #[test]
    fn capsule_fits_icon_gap_and_a_two_character_name() {
        // content = icon 15 + gap 7 + name 2 x 11 = 44
        let (w, _) = style_size(FloaterStyle::Capsule);
        let content = 15.0 + 7.0 + 2.0 * 11.0;
        assert!(
            w > content,
            "capsule {} must be wider than its content {} to leave padding",
            w,
            content
        );
    }

    #[test]
    fn capsule_slack_is_even_so_the_label_stays_centred() {
        // `justify-content: center` splits the leftover space evenly, so the
        // remainder decides the (symmetric) padding on each side. This guards
        // against picking a width that leaves an odd, lopsided amount.
        let (w, _) = style_size(FloaterStyle::Capsule);
        let content = 15.0 + 7.0 + 2.0 * 11.0; // 44
        let slack = w - content; // 28
        assert!(
            (slack / 2.0).fract() == 0.0,
            "slack {} should split into a whole number per side",
            slack
        );
        assert_eq!(slack / 2.0, 14.0, "documented padding is 14px per side");
    }

    #[test]
    fn capsule_has_room_for_a_four_character_name() {
        // All four shipped mode names are two full-width characters; the width
        // should still absorb a four-character name without ellipsising.
        let (w, _) = style_size(FloaterStyle::Capsule);
        let four_chars = 15.0 + 7.0 + 4.0 * 11.0; // 66
        assert!(
            w >= four_chars,
            "capsule {} should fit a four-character name ({})",
            w,
            four_chars
        );
    }

    #[test]
    fn capsule_and_circle_share_a_height() {
        let (_, ch) = style_size(FloaterStyle::Capsule);
        let (_, rh) = style_size(FloaterStyle::Circle);
        assert_eq!(ch, rh, "the two styles should sit on a common baseline");
    }

    #[test]
    fn floater_style_from_str_circle() {
        assert_eq!(FloaterStyle::from_str("circle"), Some(FloaterStyle::Circle));
    }

    #[test]
    fn floater_style_from_str_unknown() {
        assert_eq!(FloaterStyle::from_str("square"), None);
        assert_eq!(FloaterStyle::from_str(""), None);
        assert_eq!(FloaterStyle::from_str("Capsule"), None); // case-sensitive
    }

    #[test]
    fn floater_style_serde_round_trip() {
        let styles = [FloaterStyle::Capsule, FloaterStyle::Circle];
        for style in styles {
            let json = serde_json::to_value(style).unwrap();
            let back: FloaterStyle = serde_json::from_value(json).unwrap();
            assert_eq!(back, style);
        }
    }

    #[test]
    fn floater_status_serialization() {
        let status = FloaterStatus {
            visible: true,
            style: "circle",
            position: "top-left",
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["visible"], true);
        assert_eq!(json["style"], "circle");
        assert_eq!(json["position"], "top-left");
    }

    #[test]
    fn floater_status_serialization_hidden() {
        let status = FloaterStatus {
            visible: false,
            style: "capsule",
            position: "bottom-right",
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["visible"], false);
        assert_eq!(json["style"], "capsule");
        assert_eq!(json["position"], "bottom-right");
    }

    // ========================================================================
    // FloaterPosition
    // ========================================================================

    #[test]
    fn position_parse_all_corners() {
        assert_eq!(
            FloaterPosition::parse("top-left"),
            Some(FloaterPosition::TopLeft)
        );
        assert_eq!(
            FloaterPosition::parse("top-right"),
            Some(FloaterPosition::TopRight)
        );
        assert_eq!(
            FloaterPosition::parse("bottom-left"),
            Some(FloaterPosition::BottomLeft)
        );
        assert_eq!(
            FloaterPosition::parse("bottom-right"),
            Some(FloaterPosition::BottomRight)
        );
    }

    #[test]
    fn position_parse_rejects_unknown() {
        assert_eq!(FloaterPosition::parse("top"), None);
        assert_eq!(FloaterPosition::parse("TopLeft"), None); // kebab-case only
        assert_eq!(FloaterPosition::parse(""), None);
    }

    #[test]
    fn position_as_str_round_trips() {
        for p in [
            FloaterPosition::TopLeft,
            FloaterPosition::TopRight,
            FloaterPosition::BottomLeft,
            FloaterPosition::BottomRight,
        ] {
            assert_eq!(FloaterPosition::parse(p.as_str()), Some(p));
        }
    }

    #[test]
    fn position_serde_uses_kebab_case() {
        let json = serde_json::to_value(FloaterPosition::BottomLeft).unwrap();
        assert_eq!(json, "bottom-left");
        let back: FloaterPosition = serde_json::from_value(json).unwrap();
        assert_eq!(back, FloaterPosition::BottomLeft);
    }
}
