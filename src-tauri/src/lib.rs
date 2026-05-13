mod command_bulider;
mod front;
mod game_pad;
mod log_buffer;
mod mapping;
mod msg_sender;
mod system_status;

use front::{
    drain_logs, get_latest_command, get_settings, get_system_status, update_setting, Settings,
};
use game_pad::GamepadInterface;
use mapping::Mapper;
use msg_sender::MsgBridge;
use std::sync::{Arc, RwLock};
use std::thread;
use system_status::SystemStatus;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Arc::new(RwLock::new(Settings::default()));
    let settings_for_thread = Arc::clone(&settings);
    let latest_command: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
    let latest_command_for_thread = Arc::clone(&latest_command);
    let system_status: Arc<RwLock<Option<SystemStatus>>> = Arc::new(RwLock::new(None));
    let system_status_for_thread = Arc::clone(&system_status);

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
            let default_cfg = settings_for_thread.read().unwrap().clone();
            let mut mapper = Mapper::new(default_cfg.publish_topic.clone());
            let mut ws_url = String::new();
            let mut sub_topic = String::new();
            let mut pub_topic = default_cfg.publish_topic.clone();
            let mut ws_bridge = None;

            loop {
                interval.tick().await;
                gamepad.update();
                let s = gamepad.state();

                // 检测前端配置变化
                let cfg = settings_for_thread.read().unwrap().clone();
                let url_changed = cfg.ws_url != ws_url;
                let sub_changed = cfg.subscribe_topic != sub_topic;
                let pub_changed = cfg.publish_topic != pub_topic;

                if url_changed || sub_changed {
                    ws_url = cfg.ws_url.clone();
                    sub_topic = cfg.subscribe_topic.clone();
                    ws_bridge = Some(MsgBridge::new(&ws_url, &sub_topic));
                    log_line!("[Gamepad] ws_url={}, subscribe_topic={}", ws_url, sub_topic);
                }
                if pub_changed {
                    pub_topic = cfg.publish_topic.clone();
                    mapper = Mapper::new(pub_topic.clone());
                    log_line!("[Gamepad] publish_topic 已更新: {}", pub_topic);
                }

                // 处理接收到的消息（无论手柄是否连接）
                if let Some(ref bridge) = ws_bridge {
                    let msgs = bridge.drain_recv();
                    for msg in &msgs {
                        if let Some(status) = SystemStatus::try_from_rosbridge(msg) {
                            *system_status_for_thread.write().unwrap() = Some(status);
                        }
                    }
                }

                if !s.connected {
                    continue;
                }

                if let Some(ref bridge) = ws_bridge {
                    if mapper.apply(s, &cfg) {
                        let cmd_json = mapper.cmd.build();
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
        .manage(system_status)
        .invoke_handler(tauri::generate_handler![
            drain_logs,
            get_settings,
            update_setting,
            get_latest_command,
            get_system_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
