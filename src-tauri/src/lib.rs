mod game_pad;

use std::sync::Mutex;
use game_pad::{GamepadInterface, GamepadState};

/// Tauri 托管的手柄管理器
struct GamepadManager(Mutex<GamepadInterface>);

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 前端轮询此命令获取手柄最新状态
#[tauri::command]
fn get_gamepad_state(manager: tauri::State<'_, GamepadManager>) -> GamepadState {
    let mut iface = manager.0.lock().unwrap();
    iface.update();
    iface.state().clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化手柄接口，即使没有手柄也能正常运行（返回全零/未连接状态）
    let gamepad = GamepadInterface::new()
        .expect("failed to initialize gamepad interface");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(GamepadManager(Mutex::new(gamepad)))
        .invoke_handler(tauri::generate_handler![greet, get_gamepad_state])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
