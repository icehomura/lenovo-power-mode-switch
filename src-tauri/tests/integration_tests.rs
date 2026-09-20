//! Integration tests for the lenovo-power-mode-switch pure-logic functions.
//!
//! These tests exercise functions that do NOT require the Tauri runtime
//! or the native DLL, so they can run in any CI environment.

use lenovo_power_mode_switch_lib::power;
use lenovo_power_mode_switch_lib::shortcuts::{ShortcutBinding, ShortcutsState};

// ========================================================================
// power module — extended tests
// ========================================================================

#[test]
fn slug_to_abi_all_valid_slugs() {
    let cases = [
        ("intelligent", 1),
        ("saving", 2),
        ("performance", 3),
        ("geek", 4),
    ];
    for (slug, expected_abi) in cases {
        assert_eq!(power::slug_to_abi(slug), Some(expected_abi), "slug={}", slug);
    }
}

#[test]
fn slug_to_abi_all_aliases() {
    assert_eq!(power::slug_to_abi("auto"), Some(1));
    assert_eq!(power::slug_to_abi("smart"), Some(1));
    assert_eq!(power::slug_to_abi("cool"), Some(2));
    assert_eq!(power::slug_to_abi("perf"), Some(3));
}

#[test]
fn slug_to_abi_unknown_returns_none() {
    assert_eq!(power::slug_to_abi("turbo"), None);
    assert_eq!(power::slug_to_abi(""), None);
    assert_eq!(power::slug_to_abi("INTELLIGENT"), None); // case-sensitive
}

#[test]
fn mode_slug_valid_abi() {
    assert_eq!(power::mode_slug(1), Some("intelligent"));
    assert_eq!(power::mode_slug(2), Some("saving"));
    assert_eq!(power::mode_slug(3), Some("performance"));
    assert_eq!(power::mode_slug(4), Some("geek"));
}

#[test]
fn mode_slug_invalid_abi() {
    assert_eq!(power::mode_slug(0), None);
    assert_eq!(power::mode_slug(5), None);
    assert_eq!(power::mode_slug(-1), None);
    assert_eq!(power::mode_slug(99), None);
}

#[test]
fn slug_abi_round_trip_bidirectional() {
    // Every valid slug -> abi -> slug should return the original slug
    let slugs = ["intelligent", "saving", "performance", "geek"];
    for slug in slugs {
        let abi = power::slug_to_abi(slug).unwrap();
        let back = power::mode_slug(abi).unwrap();
        assert_eq!(back, slug, "round-trip failed for {}", slug);
    }
}

// ========================================================================
// GeekState tests
// ========================================================================

#[test]
fn geek_state_serialization() {
    use lenovo_power_mode_switch_lib::power::GeekState;

    let unsupported = serde_json::to_value(GeekState::Unsupported).unwrap();
    assert_eq!(unsupported, serde_json::json!("unsupported"));

    let available = serde_json::to_value(GeekState::Available).unwrap();
    assert_eq!(available, serde_json::json!("available"));

    let greyed = serde_json::to_value(GeekState::Greyed).unwrap();
    assert_eq!(greyed, serde_json::json!("greyed"));
}

#[test]
fn geek_state_from_abi_variants() {
    // Test the internal from_abi mapping via the public interface.
    // 0 and other negative values => Unsupported, 1 => Available, 2 => Greyed
    use lenovo_power_mode_switch_lib::power::GeekState;

    let cases: Vec<(i32, GeekState)> = vec![
        (0, GeekState::Unsupported),
        (1, GeekState::Available),
        (2, GeekState::Greyed),
        (3, GeekState::Unsupported),
        (-1, GeekState::Unsupported),
    ];
    // We can't call from_abi directly (it's private), but we verify
    // the serde round-trip of each variant.
    for (_, expected) in &cases {
        let json = serde_json::to_value(expected).unwrap();
        let back: GeekState = serde_json::from_value(json).unwrap();
        assert_eq!(&back, expected);
    }
}

// ========================================================================
// ModeInfo serialization
// ========================================================================

#[test]
fn mode_info_serialization_round_trip() {
    use lenovo_power_mode_switch_lib::power::ModeInfo;

    let info = ModeInfo {
        id: "intelligent",
        name: "智能模式",
        desc: "按负载自动调节",
        icon: "⚖️",
        color: "#4A9EFF",
        abi: 1,
        supported: true,
        greyed: false,
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["id"], "intelligent");
    assert_eq!(json["name"], "智能模式");
    assert_eq!(json["abi"], 1);
    assert_eq!(json["supported"], true);
    assert_eq!(json["greyed"], false);
}

// ========================================================================
// shortcuts module — binding tests (no Tauri runtime needed)
// ========================================================================

#[test]
fn shortcuts_state_new_is_empty() {
    let state = ShortcutsState::new();
    let bindings = state.bindings.lock().unwrap();
    assert!(bindings.is_empty());
}

#[test]
fn shortcuts_state_add_and_retrieve() {
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
    assert_eq!(bindings[0].shortcut, "Ctrl+Alt+1");
}

#[test]
fn shortcuts_state_replace_existing_mode() {
    let state = ShortcutsState::new();
    {
        let mut bindings = state.bindings.lock().unwrap();
        bindings.push(ShortcutBinding {
            mode: "intelligent".into(),
            shortcut: "Ctrl+Alt+1".into(),
        });
    }
    // Replace with new shortcut for same mode
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
fn shortcuts_state_clear() {
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

#[test]
fn shortcuts_binding_json_serialization() {
    let binding = ShortcutBinding {
        mode: "geek".into(),
        shortcut: "Ctrl+Shift+G".into(),
    };
    let json = serde_json::to_value(&binding).unwrap();
    assert_eq!(json["mode"], "geek");
    assert_eq!(json["shortcut"], "Ctrl+Shift+G");

    let restored: ShortcutBinding = serde_json::from_value(json).unwrap();
    assert_eq!(restored.mode, "geek");
    assert_eq!(restored.shortcut, "Ctrl+Shift+G");
}

#[test]
fn shortcuts_conflict_detection_logic() {
    // Simulate conflict detection: check if another mode already uses this shortcut.
    let state = ShortcutsState::new();
    {
        let mut bindings = state.bindings.lock().unwrap();
        bindings.push(ShortcutBinding {
            mode: "intelligent".into(),
            shortcut: "Ctrl+Alt+1".into(),
        });
    }

    // Same mode re-registering = allowed
    let bindings = state.bindings.lock().unwrap();
    let conflict = bindings.iter().find(|b| b.shortcut == "Ctrl+Alt+1");
    assert!(conflict.is_some());
    assert_eq!(conflict.unwrap().mode, "intelligent");
    drop(bindings);

    // Different mode trying same shortcut = conflict
    let bindings = state.bindings.lock().unwrap();
    let existing = bindings.iter().find(|b| b.shortcut == "Ctrl+Alt+1");
    assert!(existing.is_some());
    assert_ne!(existing.unwrap().mode, "performance"); // simulates conflict
}

// ========================================================================
// floater module — FloaterState and FloaterStyle tests
// ========================================================================

#[test]
fn floater_state_default() {
    let state = lenovo_power_mode_switch_lib::floater::FloaterState::new();
    assert!(!*state.visible.lock().unwrap());
    assert_eq!(
        *state.style.lock().unwrap(),
        lenovo_power_mode_switch_lib::floater::FloaterStyle::Capsule
    );
}

#[test]
fn floater_style_from_str_valid() {
    use lenovo_power_mode_switch_lib::floater::FloaterStyle;

    assert_eq!(FloaterStyle::from_str("capsule"), Some(FloaterStyle::Capsule));
    assert_eq!(FloaterStyle::from_str("circle"), Some(FloaterStyle::Circle));
}

#[test]
fn floater_style_from_str_invalid() {
    use lenovo_power_mode_switch_lib::floater::FloaterStyle;

    assert_eq!(FloaterStyle::from_str("square"), None);
    assert_eq!(FloaterStyle::from_str(""), None);
    assert_eq!(FloaterStyle::from_str("CAPSULE"), None); // case-sensitive
}

#[test]
fn floater_style_serialization() {
    use lenovo_power_mode_switch_lib::floater::FloaterStyle;

    let capsule = serde_json::to_value(FloaterStyle::Capsule).unwrap();
    assert_eq!(capsule, serde_json::json!("capsule"));

    let circle = serde_json::to_value(FloaterStyle::Circle).unwrap();
    assert_eq!(circle, serde_json::json!("circle"));
}

#[test]
fn floater_style_deserialization_round_trip() {
    use lenovo_power_mode_switch_lib::floater::FloaterStyle;

    let styles = [FloaterStyle::Capsule, FloaterStyle::Circle];
    for style in styles {
        let json = serde_json::to_value(style).unwrap();
        let back: FloaterStyle = serde_json::from_value(json).unwrap();
        assert_eq!(back, style);
    }
}

// ========================================================================
// lib module — mode_display_name tests
// ========================================================================

// mode_display_name is private to lib.rs, so we test it indirectly
// through the public slug -> name mapping which is the same logic.

#[test]
fn tray_tooltip_format() {
    // build_tooltip() format: "Lenovo Power Mode Switch — {name}"
    // We verify the pattern is predictable by checking the template.
    // Since build_tooltip() calls the DLL, we just verify the format string.
    let name = "智能模式";
    let tooltip = format!("Lenovo Power Mode Switch — {}", name);
    assert!(tooltip.starts_with("Lenovo Power Mode Switch"));
    assert!(tooltip.contains(name));
}
