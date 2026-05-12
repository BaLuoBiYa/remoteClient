mod command_bulider;
mod front;
mod game_pad;
mod log_buffer;
mod mapping;
mod msg_sender;

use front::{drain_logs, get_latest_command, get_settings, update_setting, Settings};
use game_pad::GamepadInterface;
use mapping::Mapper;
use msg_sender::MsgBridge;
use std::sync::{Arc, RwLock};
use std::thread;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Arc::new(RwLock::new(Settings::default()));
    let settings_for_thread = Arc::clone(&settings);
    let latest_command: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
    let latest_command_for_thread = Arc::clone(&latest_command);

    // ---------- 手柄后台线程 ----------
    let _gamepad_handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("failed to build tokio runtime for gamepad");

        rt.block_on(async {
            let mut gamepad =
                GamepadInterface::new().expect("failed to initialize gamepad interface");

            let mut interval = tokio::time::interval(std::time::Duration::from_millis(20));
            let mut mapper = Mapper::new("/dog/command".to_string());
            let mut ws_url = String::new();
            let mut ws_bridge = None;

            loop {
                interval.tick().await;
                gamepad.update();
                let s = gamepad.state();

                // 无论手柄是否连接，都检测 ws_url 变化以响应前端更新
                let cfg = settings_for_thread.read().unwrap().clone();
                if cfg.ws_url != ws_url {
                    ws_url = cfg.ws_url.clone();
                    ws_bridge = Some(MsgBridge::new(&ws_url));
                    log_line!("[Gamepad] ws_url 已更新: {}", ws_url);
                }

                if !s.connected {
                    continue;
                }

                if let Some(ref bridge) = ws_bridge {
                    if mapper.apply(s, &cfg) {
                        let cmd_json = mapper.cmd.build();
                        log_line!("{}", cmd_json);
                        *latest_command_for_thread.write().unwrap() = cmd_json.clone();
                        bridge.push_send(cmd_json);
                    }
                }
            }
        });
    });

    // ---------- Tauri 启动 ----------
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(settings)
        .manage(latest_command)
        .invoke_handler(tauri::generate_handler![
            drain_logs,
            get_settings,
            update_setting,
            get_latest_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
