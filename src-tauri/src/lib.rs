mod command_bulider;
mod game_pad;
mod log_buffer;
mod mapping;
mod msg_sender;

use game_pad::GamepadInterface;
use mapping::{Curve, Mapper};
use msg_sender::MsgBridge;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::thread;

/// 可从前端实时调节的参数集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub ws_url: String,
    pub curve: Curve,
    pub max_linear_speed: f32,
    pub max_angular: f32,
    pub exp_sensitivity: f32,
    pub max_base_height: f32,
    pub min_base_height: f32,
    pub max_step_height: f32,
    pub min_step_height: f32,
    pub min_duration: f32,
    pub max_duration: f32,
    pub min_button_interval_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ws_url: "ws://localhost:9090".into(),
            curve: Curve::Exponential,
            max_linear_speed: 0.1,
            max_angular: 0.1,
            exp_sensitivity: 3.0,
            max_base_height: 300.0,
            min_base_height: 80.0,
            max_step_height: 150.0,
            min_step_height: 20.0,
            min_duration: 0.1,
            max_duration: 1.0,
            min_button_interval_ms: 150,
        }
    }
}

/// 读取并清空日志缓冲区，返回所有后端日志行（读取后销毁）
#[tauri::command]
fn drain_logs() -> Vec<String> {
    log_buffer::drain_logs()
}

/// 获取当前所有参数
#[tauri::command]
fn get_settings(state: tauri::State<'_, Arc<RwLock<Settings>>>) -> Settings {
    state.read().unwrap().clone()
}

/// 前端更新单个参数
#[tauri::command]
fn update_setting(
    state: tauri::State<'_, Arc<RwLock<Settings>>>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut s = state.write().unwrap();
    match key.as_str() {
        "curve" => {
            s.curve = serde_json::from_value(value).map_err(|e| e.to_string())?;
        }
        "ws_url" => {
            s.ws_url = value.as_str().ok_or("invalid string")?.to_string();
        }
        "max_linear_speed" => s.max_linear_speed = value.as_f64().ok_or("invalid")? as f32,
        "max_angular" => s.max_angular = value.as_f64().ok_or("invalid")? as f32,
        "exp_sensitivity" => s.exp_sensitivity = value.as_f64().ok_or("invalid")? as f32,
        "max_base_height" => s.max_base_height = value.as_f64().ok_or("invalid")? as f32,
        "min_base_height" => s.min_base_height = value.as_f64().ok_or("invalid")? as f32,
        "max_step_height" => s.max_step_height = value.as_f64().ok_or("invalid")? as f32,
        "min_step_height" => s.min_step_height = value.as_f64().ok_or("invalid")? as f32,
        "min_duration" => s.min_duration = value.as_f64().ok_or("invalid")? as f32,
        "max_duration" => s.max_duration = value.as_f64().ok_or("invalid")? as f32,
        "min_button_interval_ms" => s.min_button_interval_ms = value.as_u64().ok_or("invalid")?,
        _ => return Err(format!("unknown key: {}", key)),
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Arc::new(RwLock::new(Settings::default()));
    let settings_for_thread = Arc::clone(&settings);

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
            let ws_url = settings_for_thread.read().unwrap().ws_url.clone();
            let ws_bridge = MsgBridge::new(&ws_url);

            loop {
                interval.tick().await;
                gamepad.update();
                let s = gamepad.state();
                if !s.connected {
                    continue;
                }

                let cfg = settings_for_thread.read().unwrap().clone();

                if mapper.apply(s, &cfg) {
                    log_line!("{}", mapper.cmd.build());
                    ws_bridge.push_send(mapper.cmd.build());
                }
            }
        });
    });

    // ---------- Tauri 启动 ----------
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(settings)
        .invoke_handler(tauri::generate_handler![
            drain_logs,
            get_settings,
            update_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
