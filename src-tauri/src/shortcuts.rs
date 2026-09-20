use crate::power;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_store::StoreExt;

/// Shortcut binding: which Tauri shortcut string is bound to which mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutBinding {
    pub mode: String,
    pub shortcut: String,
}

/// Central state shared across all Tauri commands via `tauri::State`.
///
/// `bindings` is public so the integration tests in `tests/` can exercise
/// add/replace/clear semantics without a live Tauri `AppHandle`.
pub struct ShortcutsState {
    pub bindings: std::sync::Mutex<Vec<ShortcutBinding>>,
}

impl ShortcutsState {
    pub fn new() -> Self {
        Self {
            bindings: std::sync::Mutex::new(Vec::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn format_modifiers(m: Modifiers) -> String {
    let mut parts = Vec::new();
    if m.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if m.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if m.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    // `Shortcut::new` normalises META to SUPER on Windows (there is no Meta key
    // there — Super is the Win key), so both spellings have to be accepted or
    // the modifier is silently dropped when a shortcut is formatted back out.
    if m.contains(Modifiers::META) || m.contains(Modifiers::SUPER) {
        parts.push("Super");
    }
    parts.join("+")
}

pub(crate) fn format_key(code: Code) -> String {
    match code {
        Code::Digit0 => "0".into(),
        Code::Digit1 => "1".into(),
        Code::Digit2 => "2".into(),
        Code::Digit3 => "3".into(),
        Code::Digit4 => "4".into(),
        Code::Digit5 => "5".into(),
        Code::Digit6 => "6".into(),
        Code::Digit7 => "7".into(),
        Code::Digit8 => "8".into(),
        Code::Digit9 => "9".into(),
        Code::KeyA => "A".into(),
        Code::KeyB => "B".into(),
        Code::KeyC => "C".into(),
        Code::KeyD => "D".into(),
        Code::KeyE => "E".into(),
        Code::KeyF => "F".into(),
        Code::KeyG => "G".into(),
        Code::KeyH => "H".into(),
        Code::KeyI => "I".into(),
        Code::KeyJ => "J".into(),
        Code::KeyK => "K".into(),
        Code::KeyL => "L".into(),
        Code::KeyM => "M".into(),
        Code::KeyN => "N".into(),
        Code::KeyO => "O".into(),
        Code::KeyP => "P".into(),
        Code::KeyQ => "Q".into(),
        Code::KeyR => "R".into(),
        Code::KeyS => "S".into(),
        Code::KeyT => "T".into(),
        Code::KeyU => "U".into(),
        Code::KeyV => "V".into(),
        Code::KeyW => "W".into(),
        Code::KeyX => "X".into(),
        Code::KeyY => "Y".into(),
        Code::KeyZ => "Z".into(),
        Code::F1 => "F1".into(),
        Code::F2 => "F2".into(),
        Code::F3 => "F3".into(),
        Code::F4 => "F4".into(),
        Code::F5 => "F5".into(),
        Code::F6 => "F6".into(),
        Code::F7 => "F7".into(),
        Code::F8 => "F8".into(),
        Code::F9 => "F9".into(),
        Code::F10 => "F10".into(),
        Code::F11 => "F11".into(),
        Code::F12 => "F12".into(),
        Code::Space => "Space".into(),
        Code::Backspace => "Backspace".into(),
        Code::Delete => "Delete".into(),
        Code::Insert => "Insert".into(),
        Code::Enter => "Enter".into(),
        Code::Tab => "Tab".into(),
        Code::Escape => "Escape".into(),
        Code::ArrowUp => "Up".into(),
        Code::ArrowDown => "Down".into(),
        Code::ArrowLeft => "Left".into(),
        Code::ArrowRight => "Right".into(),
        Code::Home => "Home".into(),
        Code::End => "End".into(),
        Code::PageUp => "PageUp".into(),
        Code::PageDown => "PageDown".into(),
        _ => format!("{:?}", code),
    }
}

pub(crate) fn format_shortcut(s: &Shortcut) -> String {
    let mut parts = Vec::new();
    let mod_str = format_modifiers(s.mods);
    if !mod_str.is_empty() {
        parts.push(mod_str);
    }
    parts.push(format_key(s.key));
    parts.join("+")
}

/// Act on a bound shortcut: switch to the named mode, or cycle to the next one.
///
/// The switch is performed here rather than handed to the frontend so that it
/// still works while the main window is hidden, and so the backend remains the
/// single source of truth for the hardware state. `broadcast_mode_change` then
/// fans the result out to every window.
fn apply_binding(app: &AppHandle, key: &str) {
    let target = if key == power::CYCLE_KEY {
        match power::next_supported_mode() {
            Ok(slug) => slug.to_string(),
            Err(e) => {
                eprintln!("[shortcut] 循环切换失败: {}", e);
                return;
            }
        }
    } else {
        key.to_string()
    };

    match power::set_mode(&target) {
        Ok(slug) => crate::broadcast_mode_change(app, slug),
        Err(e) => eprintln!("[shortcut] 切换到 {} 失败: {}", target, e),
    }
}

/// Parse a shortcut string like `"Ctrl+Alt+1"` into a Tauri `Shortcut`.
fn parse_shortcut(s: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.is_empty() {
        return Err("快捷键为空".into());
    }

    let mut mods = Modifiers::empty();
    let mut key_part = None;

    for part in &parts {
        let trimmed = part.trim();
        match trimmed.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "meta" | "win" | "cmd" | "command" => mods |= Modifiers::META,
            _ => {
                if key_part.is_some() {
                    return Err(format!("多个按键: {}", s));
                }
                key_part = Some(trimmed);
            }
        }
    }

    let key_str = key_part.ok_or_else(|| format!("缺少按键部分: {}", s))?;
    // Key names are matched case-insensitively: the recorder hands back names
    // like "F5", "Space" or "Up", while the table below is written in lower
    // case. Matching the raw string rejected every one of those.
    let code = match key_str.to_ascii_lowercase().as_str() {
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "space" => Code::Space,
        "backspace" => Code::Backspace,
        "delete" => Code::Delete,
        "insert" => Code::Insert,
        "enter" | "return" => Code::Enter,
        "tab" => Code::Tab,
        "escape" | "esc" => Code::Escape,
        "up" => Code::ArrowUp,
        "down" => Code::ArrowDown,
        "left" => Code::ArrowLeft,
        "right" => Code::ArrowRight,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        _ => return Err(format!("未知按键: {}", key_str)),
    };

    // A plain key without any modifier is almost certainly a mistake.
    if mods.is_empty() {
        return Err("快捷键必须包含至少一个修饰键 (Ctrl/Alt/Shift/Super)".into());
    }

    Ok(Shortcut::new(Some(mods), code))
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

fn persist_bindings(
    app: &AppHandle,
    state: &ShortcutsState,
) -> Result<(), String> {
    let store = app
        .store("settings.json")
        .map_err(|e| format!("打开 settings.json 失败: {}", e))?;
    let bindings = state
        .bindings
        .lock()
        .map_err(|e| format!("锁失败: {}", e))?;

    // Build the hotkeys map in the format the frontend expects.
    let hotkeys: std::collections::HashMap<&str, &str> = bindings
        .iter()
        .map(|b| (b.mode.as_str(), b.shortcut.as_str()))
        .collect();

    // Merge into the existing settings object to preserve other fields.
    let mut settings = store
        .get("settings")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    settings.insert(
        "hotkeys".to_string(),
        serde_json::to_value(&hotkeys).unwrap(),
    );
    store.set("settings", serde_json::Value::Object(settings));
    Ok(())
}

// ---------------------------------------------------------------------------
// App startup helper
// ---------------------------------------------------------------------------

/// Called once during `Builder::setup` to load persisted shortcuts from the
/// frontend's `settings.json` and register them with the OS.
/// Returns (ok_count, errors).
pub fn init_shortcuts(app: &AppHandle) -> (usize, Vec<String>) {
    let state: State<'_, ShortcutsState> = app.state();
    let mut errors = Vec::new();

    // Read from the frontend's settings store (shared storage).
    let bindings = {
        let store = match app.store("settings.json") {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("打开 settings.json 失败: {}", e));
                return (0, errors);
            }
        };
        match store.get("settings") {
            Some(val) => {
                // settings.json structure: { "settings": { "hotkeys": { "auto": "Ctrl+Alt+1", ... } } }
                if let Some(hotkeys) = val.get("hotkeys") {
                    serde_json::from_value::<std::collections::HashMap<String, String>>(hotkeys.clone())
                        .map(|map| {
                            map.into_iter()
                                .filter(|(_, v)| !v.is_empty())
                                .map(|(mode, shortcut)| ShortcutBinding { mode, shortcut })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            None => Vec::new(),
        }
    };

    let count = bindings.len();

    // Register each binding.
    for binding in &bindings {
        match parse_shortcut(&binding.shortcut) {
            Ok(shortcut) => {
                let mode = binding.mode.clone();
                let result = app.global_shortcut().on_shortcut(
                    shortcut,
                    move |_app, _shortcut, event| {
                        if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                            apply_binding(_app, &mode);
                        }
                    },
                );
                if let Err(e) = result {
                    errors.push(format!("{}: {}", binding.shortcut, e));
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", binding.shortcut, e));
            }
        }
    }

    // Persist loaded bindings into the in-memory state.
    if let Ok(mut guard) = state.bindings.lock() {
        *guard = bindings;
    }

    (count, errors)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn register_shortcut(
    mode: String,
    shortcut: String,
    state: State<'_, ShortcutsState>,
    app: AppHandle,
) -> Result<String, String> {
    let parsed = parse_shortcut(&shortcut)?;

    // Conflict detection: check if another mode already uses this shortcut.
    {
        let bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        if let Some(existing) = bindings.iter().find(|b| b.shortcut == shortcut) {
            if existing.mode != mode {
                return Err(format!(
                    "快捷键 {} 已绑定到 {}",
                    shortcut, existing.mode
                ));
            }
        }
    }

    // If this mode already has a shortcut, unregister the old one first.
    {
        let bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        if let Some(old) = bindings.iter().find(|b| b.mode == mode) {
            if let Ok(old_shortcut) = parse_shortcut(&old.shortcut) {
                let _ = app.global_shortcut().unregister(old_shortcut);
            }
        }
    }

    // Register the new shortcut.
    let mode_clone = mode.clone();
    app.global_shortcut()
        .on_shortcut(parsed, move |_app, _shortcut, event| {
            if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                apply_binding(_app, &mode_clone);
            }
        })
        .map_err(|e| format!("注册快捷键失败: {}", e))?;

    // Update state and persist.
    {
        let mut bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        bindings.retain(|b| b.mode != mode);
        bindings.push(ShortcutBinding {
            mode,
            shortcut: shortcut.clone(),
        });
    }
    persist_bindings(&app, &state)?;

    Ok(format!("已注册: {}", shortcut))
}

#[tauri::command]
pub fn unregister_shortcut(
    mode: String,
    state: State<'_, ShortcutsState>,
    app: AppHandle,
) -> Result<String, String> {
    let shortcut_str = {
        let bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        bindings
            .iter()
            .find(|b| b.mode == mode)
            .map(|b| b.shortcut.clone())
            .ok_or_else(|| format!("模式 {} 没有绑定快捷键", mode))?
    };

    let parsed = parse_shortcut(&shortcut_str)?;
    app.global_shortcut()
        .unregister(parsed)
        .map_err(|e| format!("注销快捷键失败: {}", e))?;

    {
        let mut bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        bindings.retain(|b| b.mode != mode);
    }
    persist_bindings(&app, &state)?;

    Ok(format!("已注销: {}", shortcut_str))
}

#[tauri::command]
pub fn get_shortcuts(state: State<'_, ShortcutsState>) -> Result<Vec<ShortcutBinding>, String> {
    let bindings = state
        .bindings
        .lock()
        .map_err(|e| format!("锁失败: {}", e))?;
    Ok(bindings.clone())
}

#[tauri::command]
pub fn clear_shortcuts(
    state: State<'_, ShortcutsState>,
    app: AppHandle,
) -> Result<String, String> {
    // Unregister all shortcuts at the OS level.
    {
        let bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        for binding in bindings.iter() {
            if let Ok(parsed) = parse_shortcut(&binding.shortcut) {
                let _ = app.global_shortcut().unregister(parsed);
            }
        }
    }

    // Clear state and persist.
    {
        let mut bindings = state
            .bindings
            .lock()
            .map_err(|e| format!("锁失败: {}", e))?;
        bindings.clear();
    }
    persist_bindings(&app, &state)?;

    Ok("已清除所有快捷键".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `Shortcut::new` turns a "Super/Meta" modifier into on this platform.
    ///
    /// Asserting against `Modifiers::META` directly fails on Windows, where the
    /// crate normalises it to `SUPER`; going through `Shortcut::new` keeps these
    /// tests portable.
    fn super_modifiers() -> Modifiers {
        Shortcut::new(Some(Modifiers::META), Code::KeyA).mods
    }

    // ========================================================================
    // parse_shortcut
    // ========================================================================

    #[test]
    fn parse_ctrl_letter() {
        let s = parse_shortcut("Ctrl+A").unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL);
        assert_eq!(s.key, Code::KeyA);
    }

    #[test]
    fn parse_ctrl_alt_number() {
        let s = parse_shortcut("Ctrl+Alt+1").unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(s.key, Code::Digit1);
    }

    #[test]
    fn parse_shift_fkey() {
        let s = parse_shortcut("Shift+F5").unwrap();
        assert_eq!(s.mods, Modifiers::SHIFT);
        assert_eq!(s.key, Code::F5);
    }

    #[test]
    fn parse_super_space() {
        let s = parse_shortcut("Super+Space").unwrap();
        assert_eq!(s.mods, super_modifiers());
        assert_eq!(s.key, Code::Space);
    }

    #[test]
    fn parse_meta_alias() {
        let s = parse_shortcut("Meta+Enter").unwrap();
        assert_eq!(s.mods, super_modifiers());
        assert_eq!(s.key, Code::Enter);
    }

    #[test]
    fn parse_win_alias() {
        let s = parse_shortcut("Win+X").unwrap();
        assert_eq!(s.mods, super_modifiers());
        assert_eq!(s.key, Code::KeyX);
    }

    #[test]
    fn parse_cmd_alias() {
        let s = parse_shortcut("Cmd+S").unwrap();
        assert_eq!(s.mods, super_modifiers());
        assert_eq!(s.key, Code::KeyS);
    }

    #[test]
    fn parse_command_alias() {
        let s = parse_shortcut("Command+Z").unwrap();
        assert_eq!(s.mods, super_modifiers());
        assert_eq!(s.key, Code::KeyZ);
    }

    #[test]
    fn parse_control_alias() {
        let s = parse_shortcut("Control+C").unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL);
        assert_eq!(s.key, Code::KeyC);
    }

    #[test]
    fn parse_esc_alias() {
        let s = parse_shortcut("Ctrl+Esc").unwrap();
        assert_eq!(s.key, Code::Escape);
    }

    #[test]
    fn parse_return_alias() {
        let s = parse_shortcut("Alt+Return").unwrap();
        assert_eq!(s.key, Code::Enter);
    }

    #[test]
    fn parse_four_modifiers() {
        let s = parse_shortcut("Ctrl+Alt+Shift+Super+Z").unwrap();
        assert_eq!(
            s.mods,
            Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | super_modifiers()
        );
        assert_eq!(s.key, Code::KeyZ);
    }

    #[test]
    fn parse_arrow_keys() {
        assert_eq!(parse_shortcut("Ctrl+Up").unwrap().key, Code::ArrowUp);
        assert_eq!(parse_shortcut("Ctrl+Down").unwrap().key, Code::ArrowDown);
        assert_eq!(parse_shortcut("Ctrl+Left").unwrap().key, Code::ArrowLeft);
        assert_eq!(parse_shortcut("Ctrl+Right").unwrap().key, Code::ArrowRight);
    }

    #[test]
    fn parse_navigation_keys() {
        assert_eq!(parse_shortcut("Ctrl+Home").unwrap().key, Code::Home);
        assert_eq!(parse_shortcut("Ctrl+End").unwrap().key, Code::End);
        assert_eq!(parse_shortcut("Ctrl+PageUp").unwrap().key, Code::PageUp);
        assert_eq!(parse_shortcut("Ctrl+PageDown").unwrap().key, Code::PageDown);
        assert_eq!(parse_shortcut("Ctrl+Backspace").unwrap().key, Code::Backspace);
        assert_eq!(parse_shortcut("Ctrl+Delete").unwrap().key, Code::Delete);
        assert_eq!(parse_shortcut("Ctrl+Insert").unwrap().key, Code::Insert);
        assert_eq!(parse_shortcut("Ctrl+Tab").unwrap().key, Code::Tab);
    }

    #[test]
    fn parse_letters_case_insensitive() {
        let lower = parse_shortcut("Ctrl+a").unwrap();
        let upper = parse_shortcut("Ctrl+A").unwrap();
        assert_eq!(lower.key, upper.key);
    }

    // --- Error cases ---

    #[test]
    fn parse_empty_string_fails() {
        assert!(parse_shortcut("").is_err());
    }

    #[test]
    fn parse_no_modifier_fails() {
        let err = parse_shortcut("A").unwrap_err();
        assert!(err.contains("修饰键"));
    }

    #[test]
    fn parse_unknown_key_fails() {
        assert!(parse_shortcut("Ctrl+Foo").is_err());
    }

    #[test]
    fn parse_multiple_keys_fails() {
        let err = parse_shortcut("Ctrl+A+B").unwrap_err();
        assert!(err.contains("多个按键"));
    }

    #[test]
    fn parse_modifier_only_fails() {
        assert!(parse_shortcut("Ctrl+").is_err());
    }

    // ========================================================================
    // format_modifiers
    // ========================================================================

    #[test]
    fn format_modifiers_empty() {
        assert_eq!(format_modifiers(Modifiers::empty()), "");
    }

    #[test]
    fn format_modifiers_single_ctrl() {
        assert_eq!(format_modifiers(Modifiers::CONTROL), "Ctrl");
    }

    #[test]
    fn format_modifiers_single_alt() {
        assert_eq!(format_modifiers(Modifiers::ALT), "Alt");
    }

    #[test]
    fn format_modifiers_single_shift() {
        assert_eq!(format_modifiers(Modifiers::SHIFT), "Shift");
    }

    #[test]
    fn format_modifiers_single_meta() {
        assert_eq!(format_modifiers(Modifiers::META), "Super");
    }

    #[test]
    fn format_modifiers_combined() {
        let m = Modifiers::CONTROL | Modifiers::ALT;
        assert_eq!(format_modifiers(m), "Ctrl+Alt");
    }

    // ========================================================================
    // format_key
    // ========================================================================

    #[test]
    fn format_key_digits() {
        assert_eq!(format_key(Code::Digit0), "0");
        assert_eq!(format_key(Code::Digit5), "5");
        assert_eq!(format_key(Code::Digit9), "9");
    }

    #[test]
    fn format_key_letters() {
        assert_eq!(format_key(Code::KeyA), "A");
        assert_eq!(format_key(Code::KeyZ), "Z");
    }

    #[test]
    fn format_key_fkeys() {
        assert_eq!(format_key(Code::F1), "F1");
        assert_eq!(format_key(Code::F12), "F12");
    }

    #[test]
    fn format_key_special() {
        assert_eq!(format_key(Code::Space), "Space");
        assert_eq!(format_key(Code::Enter), "Enter");
        assert_eq!(format_key(Code::Escape), "Escape");
        assert_eq!(format_key(Code::Tab), "Tab");
    }

    #[test]
    fn format_key_arrows() {
        assert_eq!(format_key(Code::ArrowUp), "Up");
        assert_eq!(format_key(Code::ArrowDown), "Down");
        assert_eq!(format_key(Code::ArrowLeft), "Left");
        assert_eq!(format_key(Code::ArrowRight), "Right");
    }

    // ========================================================================
    // format_shortcut round-trip
    // ========================================================================

    #[test]
    fn format_shortcut_round_trip() {
        let cases = ["Ctrl+Alt+1", "Shift+F5", "Super+Space", "Ctrl+X"];
        for input in cases {
            let parsed = parse_shortcut(input).unwrap();
            let formatted = format_shortcut(&parsed);
            let reparsed = parse_shortcut(&formatted).unwrap();
            assert_eq!(parsed, reparsed, "round-trip failed for: {}", input);
        }
    }

    #[test]
    fn format_shortcut_ctrl_a() {
        let s = parse_shortcut("Ctrl+A").unwrap();
        assert_eq!(format_shortcut(&s), "Ctrl+A");
    }

    #[test]
    fn format_shortcut_super_enter() {
        let s = parse_shortcut("Super+Enter").unwrap();
        assert_eq!(format_shortcut(&s), "Super+Enter");
    }

    // ========================================================================
    // ShortcutBinding serialization
    // ========================================================================

    #[test]
    fn binding_serialize_deserialize() {
        let binding = ShortcutBinding {
            mode: "intelligent".into(),
            shortcut: "Ctrl+Alt+1".into(),
        };
        let json = serde_json::to_string(&binding).unwrap();
        let restored: ShortcutBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.mode, "intelligent");
        assert_eq!(restored.shortcut, "Ctrl+Alt+1");
    }

    #[test]
    fn binding_json_structure() {
        let binding = ShortcutBinding {
            mode: "saving".into(),
            shortcut: "Ctrl+Alt+2".into(),
        };
        let json = serde_json::to_value(&binding).unwrap();
        assert_eq!(json["mode"], "saving");
        assert_eq!(json["shortcut"], "Ctrl+Alt+2");
    }

    // ========================================================================
    // ShortcutsState
    // ========================================================================

    #[test]
    fn state_new_is_empty() {
        let state = ShortcutsState::new();
        let bindings = state.bindings.lock().unwrap();
        assert!(bindings.is_empty());
    }

    #[test]
    fn state_add_and_retrieve() {
        let state = ShortcutsState::new();
        {
            let mut bindings = state.bindings.lock().unwrap();
            bindings.push(ShortcutBinding {
                mode: "intelligent".into(),
                shortcut: "Ctrl+Alt+1".into(),
            });
        }
        let bindings = state.bindings.lock().unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].mode, "intelligent");
    }

    #[test]
    fn state_replace_existing_mode() {
        let state = ShortcutsState::new();
        {
            let mut bindings = state.bindings.lock().unwrap();
            bindings.push(ShortcutBinding {
                mode: "intelligent".into(),
                shortcut: "Ctrl+Alt+1".into(),
            });
        }
        {
            let mut bindings = state.bindings.lock().unwrap();
            bindings.retain(|b| b.mode != "intelligent");
            bindings.push(ShortcutBinding {
                mode: "intelligent".into(),
                shortcut: "Ctrl+Alt+9".into(),
            });
        }
        let bindings = state.bindings.lock().unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].shortcut, "Ctrl+Alt+9");
    }

    #[test]
    fn state_clear() {
        let state = ShortcutsState::new();
        {
            let mut bindings = state.bindings.lock().unwrap();
            bindings.push(ShortcutBinding {
                mode: "saving".into(),
                shortcut: "Ctrl+Alt+2".into(),
            });
            bindings.push(ShortcutBinding {
                mode: "performance".into(),
                shortcut: "Ctrl+Alt+3".into(),
            });
        }
        {
            let mut bindings = state.bindings.lock().unwrap();
            bindings.clear();
        }
        let bindings = state.bindings.lock().unwrap();
        assert!(bindings.is_empty());
    }
}
