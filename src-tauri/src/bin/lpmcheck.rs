// Headless diagnostic for the isolated power bridge.
//
// Cargo places this binary in `target/debug/` - the same directory tauri-build
// stages the bridge into - so it exercises the exact same FFI path the GUI uses
// and can be scripted, unlike the GUI itself.
//
//   cargo run --bin lpmcheck          # read-only: capabilities + current mode
//   cargo run --bin lpmcheck -- perf  # also switch to a mode and read back
use lenovo_power_mode_switch_lib::power;

fn main() {
    let mode_arg = std::env::args().nth(1);

    println!("=== Lenovo Power Bridge 诊断 (Rust FFI) ===");

    if let Err(e) = power::get_client() {
        eprintln!("[FAIL] 桥接不可用: {}", e);
        std::process::exit(1);
    }
    println!("[ OK ] lpm_bridge.dll 已加载并初始化");

    match power::capability() {
        Ok(caps) => println!("[ OK ] 硬件能力位掩码: 0x{:X}", caps),
        Err(e) => println!("[FAIL] 能力查询: {}", e),
    }

    let modes = match power::get_modes() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[FAIL] 支持模式查询: {}", e);
            std::process::exit(1);
        }
    };
    for m in &modes {
        let mark = if m.greyed {
            "已置灰"
        } else if m.supported {
            "支持"
        } else {
            "不支持"
        };
        println!(
            "       {} {:<12} {:<8} {} (abi={})",
            m.icon, m.id, mark, m.name, m.abi
        );
    }

    match power::geek_state() {
        Ok(power::GeekState::Available) => println!("[ OK ] 极客模式: 可用"),
        Ok(power::GeekState::Greyed) => println!("[WARN] 极客模式: 硬件支持，但当前被系统置灰"),
        Ok(power::GeekState::Unsupported) => println!("[ OK ] 极客模式: 本机硬件不支持"),
        Err(e) => println!("[FAIL] 极客模式: {}", e),
    }

    match power::get_current_mode() {
        Ok(slug) => println!("[ OK ] 当前模式: {}", slug),
        Err(e) => println!("[FAIL] 当前模式: {}", e),
    }

    let Some(target) = mode_arg else { return };

    println!("--- 切换到 {} ---", target);
    match power::set_mode(&target) {
        Ok(applied) => println!("[ OK ] 已切换到: {} (硬件回读一致)", applied),
        Err(e) => {
            eprintln!("[FAIL] {}", e);
            std::process::exit(1);
        }
    }
}
