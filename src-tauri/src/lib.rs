pub mod floater;
pub mod power;
pub mod shortcuts;

use power::{GeekState, ModeInfo};
use shortcuts::ShortcutsState;
use std::sync::Mutex;
use std::thread;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent, Wry,
};

// ---------------------------------------------------------------------------
// Shared state: tracks last-known hardware mode for polling comparison
// ---------------------------------------------------------------------------

struct AppState {
    current_mode: Mutex<String>,
}

/// Broadcast `mode-changed` to every window and update cached state.
pub(crate) fn broadcast_mode_change(app: &AppHandle, slug: &str) {
    let _ = app.emit("mode-changed", slug);
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cur) = state.current_mode.lock() {
            *cur = slug.to_string();
        }
    }
    sync_tray(app, slug);
}

/// Read hardware once, emit event if mode differs from cache. Returns true on change.
fn poll_hardware_once(app: &AppHandle) -> bool {
    let slug = match power::get_current_mode() {
        Ok(s) => s.to_string(),
        Err(_) => return false,
    };
    let changed = if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cur) = state.current_mode.lock() {
            if *cur != slug {
                *cur = slug.clone();
                true
            } else {
                false
            }
        } else {
            true
        }
    } else {
        true
    };
    if changed {
        let _ = app.emit("mode-changed", &slug);
        sync_tray(app, &slug);
    }
    changed
}

// ---------------------------------------------------------------------------
// Tray bookkeeping
// ---------------------------------------------------------------------------

/// Handles for the tray entries that need updating after the fact.
struct TrayMenu {
    /// (mode slug, item) pairs, kept so the active one can be ticked.
    mode_items: Mutex<Vec<(String, CheckMenuItem<Wry>)>>,
    /// The show/hide-floater entry, whose label tracks the window's state.
    floater_item: Mutex<Option<MenuItem<Wry>>>,
}

impl TrayMenu {
    fn new() -> Self {
        Self {
            mode_items: Mutex::new(Vec::new()),
            floater_item: Mutex::new(None),
        }
    }
}

/// Tick the active mode and refresh the tooltip.
///
/// Every path that can change the mode funnels through here — tray, global
/// shortcut, floater click and the hardware polling thread — so the tray can
/// never drift out of step with the hardware.
fn sync_tray(app: &AppHandle, slug: &str) {
    if let Some(menu) = app.try_state::<TrayMenu>() {
        if let Ok(items) = menu.mode_items.lock() {
            for (id, item) in items.iter() {
                let _ = item.set_checked(id == slug);
            }
        }
    }
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&build_tooltip()));
    }
}

/// Keep the floater entry's label in step with the window's actual state.
fn sync_tray_floater(app: &AppHandle) {
    let visible = app
        .get_webview_window(floater::FLOATER_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if let Some(menu) = app.try_state::<TrayMenu>() {
        if let Ok(guard) = menu.floater_item.lock() {
            if let Some(item) = guard.as_ref() {
                let _ = item.set_text(if visible { "隐藏悬浮窗" } else { "显示悬浮窗" });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_modes() -> Result<Vec<ModeInfo>, String> {
    power::get_modes()
}

#[tauri::command]
fn get_current_mode() -> Result<String, String> {
    power::get_current_mode().map(|s| s.to_string())
}

/// Switch mode by slug, then broadcast to all windows.
#[tauri::command]
fn set_mode(mode: String, app: AppHandle) -> Result<String, String> {
    let slug = power::set_mode(&mode)?;
    broadcast_mode_change(&app, slug);
    Ok(slug.to_string())
}

#[tauri::command]
fn check_connection() -> String {
    match power::get_client() {
        Ok(()) => "connected".to_string(),
        Err(e) => format!("disconnected: {}", e),
    }
}

/// Cycle to the next hardware-supported mode, then broadcast the change.
///
/// This is the single source of truth for cycle logic — called by the floater
/// click handler, the cycle shortcut, and the tray menu.
#[tauri::command]
fn cycle_mode(app: AppHandle) -> Result<String, String> {
    let slug = power::next_supported_mode()?;
    let slug = power::set_mode(slug)?;
    broadcast_mode_change(&app, slug);
    Ok(slug.to_string())
}

#[tauri::command]
fn get_capability() -> Result<i32, String> {
    power::capability()
}

#[tauri::command]
fn get_geek_state() -> Result<GeekState, String> {
    power::geek_state()
}

// ---------------------------------------------------------------------------
// Settings persistence (via tauri-plugin-store)
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<serde_json::Value, String> {
    use tauri_plugin_store::StoreExt;
    let store = app
        .store("settings.json")
        .map_err(|e| format!("打开 settings.json 失败: {}", e))?;
    Ok(store
        .get("settings")
        .unwrap_or(serde_json::json!({})))
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: serde_json::Value) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app
        .store("settings.json")
        .map_err(|e| format!("打开 settings.json 失败: {}", e))?;
    store.set("settings", settings);
    store
        .save()
        .map_err(|e| format!("保存 settings.json 失败: {}", e))?;
    Ok(())
}

/// Slug -> Chinese display name for tray tooltip
fn mode_display_name(slug: &str) -> &'static str {
    match slug {
        "intelligent" => "智能模式",
        "saving" => "省电模式",
        "performance" => "性能模式",
        "geek" => "极客模式",
        _ => "未知模式",
    }
}

/// Read the current mode and format the tray tooltip string.
fn build_tooltip() -> String {
    let name = power::get_current_mode()
        .map(mode_display_name)
        .unwrap_or("未知模式");
    format!("Lenovo Power Mode Switch — {}", name)
}

/// Initial mode read used at startup to seed the AppState cache.
fn read_initial_mode() -> String {
    power::get_current_mode()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Hardware polling interval in milliseconds.
const POLL_INTERVAL_MS: u64 = 3000;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance must be registered first: a second launch would
        // otherwise start a rival process fighting over the same hardware.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(ShortcutsState::new())
        .manage(floater::FloaterState::new())
        .manage(TrayMenu::new())
        .manage(AppState {
            current_mode: Mutex::new(read_initial_mode()),
        })
        .invoke_handler(tauri::generate_handler![
            get_modes,
            get_current_mode,
            set_mode,
            cycle_mode,
            check_connection,
            get_capability,
            get_geek_state,
            shortcuts::register_shortcut,
            shortcuts::unregister_shortcut,
            shortcuts::get_shortcuts,
            shortcuts::clear_shortcuts,
            floater::show_floater,
            floater::hide_floater,
            floater::toggle_floater,
            floater::destroy_floater,
            floater::set_floater_style,
            floater::set_floater_position,
            floater::get_floater_status,
            get_settings,
            save_settings,
        ])
        .setup(|app| {
            // --- Hardware polling thread ------------------------------------
            let handle = app.handle().clone();
            thread::spawn(move || loop {
                thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));
                poll_hardware_once(&handle);
            });

            // --- Tray menu ---------------------------------------------------
            // Modes are check items so the active one is visible at a glance.
            let current = power::get_current_mode().unwrap_or("intelligent");
            let intelligent = CheckMenuItem::with_id(app, "intelligent", "智能模式", true, current == "intelligent", None::<&str>)?;
            let saving      = CheckMenuItem::with_id(app, "saving", "省电模式", true, current == "saving", None::<&str>)?;
            let performance = CheckMenuItem::with_id(app, "performance", "性能模式", true, current == "performance", None::<&str>)?;
            let geek        = CheckMenuItem::with_id(app, "geek", "极客模式", true, current == "geek", None::<&str>)?;
            let sep         = PredefinedMenuItem::separator(app)?;
            let floater_it  = MenuItem::with_id(app, "floater", "显示悬浮窗", true, None::<&str>)?;
            let show        = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let quit        = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &intelligent,
                    &saving,
                    &performance,
                    &geek,
                    &sep,
                    &floater_it,
                    &show,
                    &quit,
                ],
            )?;

            // Keep handles so the ticks and the floater label can be updated later.
            {
                let tray_state = app.state::<TrayMenu>();

                let mut items = tray_state
                    .mode_items
                    .lock()
                    .map_err(|e| format!("模式菜单锁失败: {}", e))?;
                items.push(("intelligent".into(), intelligent.clone()));
                items.push(("saving".into(), saving.clone()));
                items.push(("performance".into(), performance.clone()));
                items.push(("geek".into(), geek.clone()));
                drop(items);

                let mut floater_slot = tray_state
                    .floater_item
                    .lock()
                    .map_err(|e| format!("悬浮窗菜单锁失败: {}", e))?;
                *floater_slot = Some(floater_it.clone());
            }

            // --- Tray icon ---------------------------------------------------
            let _tray = TrayIconBuilder::with_id("main-tray")
                .tooltip(&build_tooltip())
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "intelligent" | "saving" | "performance" | "geek" => {
                        let slug = event.id.as_ref();
                        if let Err(e) = power::set_mode(slug) {
                            eprintln!("[tray] 切换模式失败: {}", e);
                            return;
                        }
                        // Updates the tick, the tooltip and every open window.
                        broadcast_mode_change(app, slug);
                    }
                    "floater" => {
                        // Spawned: this handler runs on the main thread, and
                        // showing the floater may create a webview window,
                        // which deadlocks there (see floater.rs).
                        let handle = app.clone();
                        thread::spawn(move || {
                            match floater::toggle_floater_sync(&handle) {
                                Ok(_) => sync_tray_floater(&handle),
                                Err(e) => eprintln!("[tray] 切换悬浮窗失败: {}", e),
                            }
                        });
                    }
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // --- Restore the floating window the user left enabled ----------
            floater::restore_from_settings(app.handle());
            // ...and reflect that in the tray entry's label.
            sync_tray_floater(app.handle());

            // --- Global shortcuts -------------------------------------------
            let handle = app.handle().clone();
            let (_count, errors) = shortcuts::init_shortcuts(&handle);
            if !errors.is_empty() {
                eprintln!("[shortcut] 加载警告: {:?}", errors);
            }

            // --- Window lifecycle: hide on close instead of exit -----------
            let handle_clone = app.handle().clone();
            for label in ["main", "floater"] {
                if let Some(win) = app.get_webview_window(label) {
                    let h = handle_clone.clone();
                    win.on_window_event(move |event| {
                        if let WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            if let Some(w) = h.get_webview_window(label) {
                                let _ = w.hide();
                            }
                            if label == "floater" {
                                if let Ok(mut vis) = h.state::<floater::FloaterState>().visible.lock() {
                                    *vis = false;
                                }
                            }
                        }
                    });
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
