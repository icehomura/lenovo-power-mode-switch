use libloading::Library;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// The bridge is a self-contained C++/CLI mixed-mode DLL that drives an
/// ISOLATED copy of Lenovo's IdeaNotebookAddin / PowerBattery native layer.
/// It does not touch the installed Lenovo Vantage service and needs no RPC or
/// signature-trust handshake.
///
/// `lpm_bridge.dll` and its sibling addin DLLs must sit next to the executable.
const BRIDGE_DLL: &str = "lpm_bridge.dll";

type InitFn = unsafe extern "C" fn() -> i32;
type SetModeFn = unsafe extern "C" fn(i32) -> i32;
type GetModeFn = unsafe extern "C" fn() -> i32;
type CapabilityFn = unsafe extern "C" fn() -> i32;
type SupportedFn = unsafe extern "C" fn() -> i32;
type GeekStateFn = unsafe extern "C" fn() -> i32;
type LastErrorCopyFn = unsafe extern "C" fn(*mut u8, i32) -> i32;

pub struct PowerClient {
    _lib: Library,
    init: InitFn,
    set_mode: SetModeFn,
    get_mode: GetModeFn,
    capability: CapabilityFn,
    supported: SupportedFn,
    geek_state: GeekStateFn,
    last_error: LastErrorCopyFn,
}

/// Locate the bridge DLL: prefer the executable's directory so we always use
/// our own isolated copy, never anything from the Lenovo install tree.
fn bridge_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(BRIDGE_DLL);
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from(BRIDGE_DLL)
}

impl PowerClient {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let path = bridge_path();
            let lib = Library::new(&path)
                .map_err(|e| format!("加载 {} 失败: {}", path.display(), e))?;

            let init: InitFn = *lib
                .get(b"lpm_init\0")
                .map_err(|e| format!("缺少 lpm_init: {}", e))?;
            let set_mode: SetModeFn = *lib
                .get(b"lpm_set_mode\0")
                .map_err(|e| format!("缺少 lpm_set_mode: {}", e))?;
            let get_mode: GetModeFn = *lib
                .get(b"lpm_get_mode\0")
                .map_err(|e| format!("缺少 lpm_get_mode: {}", e))?;
            let capability: CapabilityFn = *lib
                .get(b"lpm_capability\0")
                .map_err(|e| format!("缺少 lpm_capability: {}", e))?;
            let supported: SupportedFn = *lib
                .get(b"lpm_supported_modes\0")
                .map_err(|e| format!("缺少 lpm_supported_modes: {}", e))?;
            let geek_state: GeekStateFn = *lib
                .get(b"lpm_geek_state\0")
                .map_err(|e| format!("缺少 lpm_geek_state: {}", e))?;
            let last_error: LastErrorCopyFn = *lib
                .get(b"lpm_last_error_copy\0")
                .map_err(|e| format!("缺少 lpm_last_error_copy: {}", e))?;

            let client = PowerClient {
                _lib: lib,
                init,
                set_mode,
                get_mode,
                capability,
                supported,
                geek_state,
                last_error,
            };

            let rc = (client.init)();
            if rc != 0 {
                return Err(client.error_or(format!("lpm_init 返回 {}", rc)));
            }

            Ok(client)
        }
    }

    fn error_or(&self, fallback: String) -> String {
        let mut buf = [0u8; 1024];
        let n = unsafe { (self.last_error)(buf.as_mut_ptr(), buf.len() as i32) };
        if n <= 0 {
            return fallback;
        }
        let n = (n as usize).min(buf.len());
        match std::str::from_utf8(&buf[..n]) {
            Ok(s) if !s.is_empty() => s.to_string(),
            _ => fallback,
        }
    }

    fn set_mode(&self, mode: i32) -> Result<(), String> {
        if !(1..=4).contains(&mode) {
            return Err(format!("模式超出范围: {}", mode));
        }
        let rc = unsafe { (self.set_mode)(mode) };
        if rc != 0 {
            return Err(self.error_or(format!("lpm_set_mode 返回 {}", rc)));
        }
        Ok(())
    }

    fn get_mode(&self) -> Result<i32, String> {
        let v = unsafe { (self.get_mode)() };
        if v < 0 {
            return Err(self.error_or("读取当前模式失败".into()));
        }
        Ok(v)
    }

    fn capability(&self) -> Result<i32, String> {
        let v = unsafe { (self.capability)() };
        if v < 0 {
            return Err(self.error_or("读取硬件能力失败".into()));
        }
        Ok(v)
    }

    fn supported_modes(&self) -> Result<i32, String> {
        let v = unsafe { (self.supported)() };
        if v < 0 {
            return Err(self.error_or("读取支持模式失败".into()));
        }
        Ok(v)
    }

    fn geek_state(&self) -> Result<GeekState, String> {
        let v = unsafe { (self.geek_state)() };
        if v < 0 {
            return Err(self.error_or("读取极客模式状态失败".into()));
        }
        Ok(GeekState::from_abi(v))
    }
}

/// Whether Geek mode is usable, distinguishing "hardware cannot" from
/// "hardware can, but the machine greys it out right now".
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum GeekState {
    Unsupported,
    Available,
    Greyed,
}

impl GeekState {
    fn from_abi(v: i32) -> Self {
        match v {
            1 => GeekState::Available,
            2 => GeekState::Greyed,
            _ => GeekState::Unsupported,
        }
    }
}

/// One switchable power profile, as surfaced to the frontend.
#[derive(Serialize, Clone)]
pub struct ModeInfo {
    /// Stable slug used by the UI and by `set_mode`.
    pub id: &'static str,
    /// Display name (Chinese).
    pub name: &'static str,
    /// Short display description.
    pub desc: &'static str,
    pub icon: &'static str,
    pub color: &'static str,
    /// Bridge ABI id (1..4), kept for diagnostics.
    pub abi: i32,
    /// Whether this machine's hardware reports support for the mode.
    pub supported: bool,
    /// Supported, but the vendor stack greys it out right now (Geek only).
    pub greyed: bool,
}

/// The four modes in canonical order: slug, Chinese name, description,
/// icon, colour, ABI id.
const MODES: [(&str, &str, &str, &str, &str, i32); 4] = [
    (
        "intelligent",
        "智能模式",
        "按负载自动调节",
        "⚖️",
        "#4A9EFF",
        1,
    ),
    ("saving", "省电模式", "风扇更安静、功耗更低", "🍃", "#34C759", 2),
    ("performance", "性能模式", "释放持续性能", "⚡", "#FF9F0A", 3),
    ("geek", "极客模式", "最高性能，需硬件支持", "🚀", "#FF3B30", 4),
];

/// Key used for the "cycle through the modes" shortcut binding.
///
/// Kept alongside the mode slugs so both kinds of binding can share a single
/// `ShortcutBinding` list instead of needing a parallel structure.
pub const CYCLE_KEY: &str = "cycle";

/// Maps the bridge ABI mode id to our public slug.
pub fn mode_slug(abi: i32) -> Option<&'static str> {
    MODES
        .iter()
        .find(|(_, _, _, _, _, id)| *id == abi)
        .map(|(slug, ..)| *slug)
}

/// Maps a public slug to the bridge ABI mode id.
/// A few aliases are accepted so the CLI's vocabulary also works here.
pub fn slug_to_abi(slug: &str) -> Option<i32> {
    match slug {
        "intelligent" | "auto" | "smart" => Some(1),
        "saving" | "cool" => Some(2),
        "performance" | "perf" => Some(3),
        "geek" => Some(4),
        _ => None,
    }
}

/// Bit this mode occupies in the hardware capability bitmask
/// (mirrors Lenovo's SupportedMmcModeType flags — note these are NOT contiguous).
fn supported_bit(abi: i32) -> i32 {
    match abi {
        1 => 0x01, // MMC_Auto
        2 => 0x02, // MMC_Cool
        3 => 0x08, // MMC_Performance
        4 => 0x10, // MMC_Geek
        _ => 0,
    }
}

static CLIENT: Mutex<Option<PowerClient>> = Mutex::new(None);

/// Run `f` with the shared client, initialising it on first use.
fn with_client<T>(f: impl FnOnce(&PowerClient) -> Result<T, String>) -> Result<T, String> {
    let mut guard = CLIENT.lock().map_err(|e| format!("锁失败: {}", e))?;
    if guard.is_none() {
        *guard = Some(PowerClient::new()?);
    }
    f(guard.as_ref().expect("client initialised above"))
}

/// Load the bridge and verify it can talk to the hardware layer.
/// Used by the UI as a connectivity probe.
pub fn get_client() -> Result<(), String> {
    with_client(|c| c.capability().map(|_| ()))
}

/// All four modes, each flagged with whether the hardware supports it and
/// whether the vendor stack has greyed it out.
///
/// The bridge announces a Geek-capable UI (UISupportGeekMode) on every request;
/// without that flag the driver omits MMC_Geek from the returned mask even on
/// hardware that supports it.
pub fn get_modes() -> Result<Vec<ModeInfo>, String> {
    with_client(|c| {
        let mask = c.supported_modes()?;
        let geek = c.geek_state().unwrap_or(GeekState::Unsupported);

        let modes = MODES
            .iter()
            .map(|(slug, name, desc, icon, color, abi)| {
                let supported = mask & supported_bit(*abi) != 0;
                ModeInfo {
                    id: slug,
                    name,
                    desc,
                    icon,
                    color,
                    abi: *abi,
                    supported,
                    greyed: *abi == 4 && geek == GeekState::Greyed,
                }
            })
            .collect();
        Ok(modes)
    })
}

/// The mode currently reported by the hardware, as a slug.
pub fn get_current_mode() -> Result<&'static str, String> {
    with_client(|c| {
        let abi = c.get_mode()?;
        mode_slug(abi).ok_or_else(|| format!("硬件返回未知模式 id: {}", abi))
    })
}

/// Pick the next selectable mode after `current`, in canonical order.
///
/// Split out from the hardware call so the ordering and the gating rules can be
/// tested without the bridge DLL present.
fn next_selectable(modes: &[ModeInfo], current: &str) -> Option<&'static str> {
    let idx = modes.iter().position(|m| m.id == current)?;
    // `1..len` deliberately excludes the current mode, so a machine where the
    // current mode is the only usable one reports "nowhere to go" instead of
    // cycling back onto itself.
    (1..modes.len())
        .map(|step| &modes[(idx + step) % modes.len()])
        .find(|m| m.supported && !m.greyed)
        .map(|m| m.id)
}

/// The next mode the hardware will actually accept, for the cycle shortcut.
pub fn next_supported_mode() -> Result<&'static str, String> {
    let current = get_current_mode()?;
    let modes = get_modes()?;
    next_selectable(&modes, current).ok_or_else(|| "没有其他可切换的模式".to_string())
}

/// Switch to `slug`, verifying the hardware actually accepted it.
pub fn set_mode(slug: &str) -> Result<&'static str, String> {
    let abi = slug_to_abi(slug).ok_or_else(|| format!("未知模式: {}", slug))?;

    with_client(|c| {
        let mask = c.supported_modes()?;
        if mask & supported_bit(abi) == 0 {
            return Err(format!("本机硬件不支持该模式: {}", slug));
        }
        c.set_mode(abi)
    })?;

    // Read back so the UI never reports a switch the hardware did not take.
    let actual = get_current_mode()?;
    if actual != slug {
        return Err(format!(
            "硬件未接受切换：请求 {}，实际 {}",
            slug, actual
        ));
    }
    Ok(actual)
}

/// Human-readable capability bitmask for the diagnostics panel.
pub fn capability() -> Result<i32, String> {
    with_client(|c| c.capability())
}

/// Whether Geek mode is usable on this machine.
pub fn geek_state() -> Result<GeekState, String> {
    with_client(|c| c.geek_state())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip_through_abi() {
        for (slug, _, _, _, _, abi) in MODES {
            assert_eq!(slug_to_abi(slug), Some(abi));
            assert_eq!(mode_slug(abi), Some(slug));
        }
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(slug_to_abi("auto"), Some(1));
        assert_eq!(slug_to_abi("cool"), Some(2));
        assert_eq!(slug_to_abi("perf"), Some(3));
        assert_eq!(slug_to_abi("nonsense"), None);
    }

    #[test]
    fn mode_slugs_are_unique() {
        for (i, (slug, ..)) in MODES.iter().enumerate() {
            for (other, ..) in MODES.iter().skip(i + 1) {
                assert_ne!(slug, other);
            }
        }
    }

    #[test]
    fn abi_aliases_share_bit() {
        // Geek is 0x10, not 0x04 — the bitmask is the non-contiguous one.
        assert_eq!(supported_bit(3), 0x08);
        assert_eq!(supported_bit(4), 0x10);
    }

    // ========================================================================
    // next_selectable
    // ========================================================================

    fn info(id: &'static str, supported: bool, greyed: bool) -> ModeInfo {
        ModeInfo {
            id,
            name: "",
            desc: "",
            icon: "",
            color: "",
            abi: 0,
            supported,
            greyed,
        }
    }

    fn all_modes() -> Vec<ModeInfo> {
        vec![
            info("intelligent", true, false),
            info("saving", true, false),
            info("performance", true, false),
            info("geek", true, false),
        ]
    }

    #[test]
    fn next_selectable_moves_forward() {
        assert_eq!(next_selectable(&all_modes(), "intelligent"), Some("saving"));
        assert_eq!(next_selectable(&all_modes(), "saving"), Some("performance"));
    }

    #[test]
    fn next_selectable_wraps_around() {
        assert_eq!(next_selectable(&all_modes(), "geek"), Some("intelligent"));
    }

    #[test]
    fn next_selectable_skips_unsupported() {
        let mut modes = all_modes();
        modes[1].supported = false; // saving
        assert_eq!(next_selectable(&modes, "intelligent"), Some("performance"));
    }

    #[test]
    fn next_selectable_skips_greyed() {
        let mut modes = all_modes();
        modes[3].greyed = true; // geek
        assert_eq!(next_selectable(&modes, "performance"), Some("intelligent"));
    }

    #[test]
    fn next_selectable_returns_none_when_nothing_else_is_usable() {
        let mut modes = all_modes();
        modes[1].supported = false;
        modes[2].supported = false;
        modes[3].supported = false;
        // Only the current mode is usable, so there is nowhere to cycle to.
        assert_eq!(next_selectable(&modes, "intelligent"), None);
    }

    #[test]
    fn next_selectable_rejects_unknown_current() {
        assert_eq!(next_selectable(&all_modes(), "turbo"), None);
    }
}
