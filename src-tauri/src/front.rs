use crate::log_buffer;
use crate::mapping::Curve;
use crate::system_status::SystemStatus;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// 可从前端实时调节的参数集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub ws_url: String,
    pub publish_topic: String,
    pub subscribe_topic: String,
    pub curve: Curve,
    pub max_linear_speed_x: f32,
    pub max_linear_speed_y: f32,
    pub max_roll: f32,
    pub max_pitch: f32,
    pub exp_sensitivity: f32,
    pub max_base_height: f32,
    pub min_base_height: f32,
    pub max_step_height: f32,
    pub min_step_height: f32,
    pub min_duration: f32,
    pub max_duration: f32,
    pub min_button_interval_ms: u64,
    pub step: f32,
    /// 摇杆轴低通滤波系数 (0~1)，越小越平滑，0 不过滤
    pub axis_smooth: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ws_url: "ws://localhost:9090".into(),
            publish_topic: "/dog/command".into(),
            subscribe_topic: "/dog/monitor".into(),
            curve: Curve::Exponential,
            max_linear_speed_x: 0.1,
            max_linear_speed_y: 0.1,
            max_roll: 0.1,
            max_pitch: 0.1,
            exp_sensitivity: 3.0,
            max_base_height: 300.0,
            min_base_height: 80.0,
            max_step_height: 150.0,
            min_step_height: 20.0,
            min_duration: 0.1,
            max_duration: 1.0,
            min_button_interval_ms: 150,
            step: 10.0,
            axis_smooth: 0.3,
        }
    }
}

/// 读取并清空日志缓冲区，返回所有后端日志行（读取后销毁）
#[tauri::command]
pub fn drain_logs() -> Vec<String> {
    log_buffer::drain_logs()
}

/// 获取当前所有参数
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, Arc<RwLock<Settings>>>) -> Settings {
    state.read().unwrap().clone()
}

/// 前端更新单个参数
#[tauri::command]
pub fn update_setting(
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
        "publish_topic" => {
            s.publish_topic = value.as_str().ok_or("invalid string")?.to_string();
        }
        "subscribe_topic" => {
            s.subscribe_topic = value.as_str().ok_or("invalid string")?.to_string();
        }
        "max_linear_speed_x" => s.max_linear_speed_x = value.as_f64().ok_or("invalid")? as f32,
        "max_linear_speed_y" => s.max_linear_speed_y = value.as_f64().ok_or("invalid")? as f32,
        "max_roll" => s.max_roll = value.as_f64().ok_or("invalid")? as f32,
        "max_pitch" => s.max_pitch = value.as_f64().ok_or("invalid")? as f32,
        "exp_sensitivity" => s.exp_sensitivity = value.as_f64().ok_or("invalid")? as f32,
        "max_base_height" => s.max_base_height = value.as_f64().ok_or("invalid")? as f32,
        "min_base_height" => s.min_base_height = value.as_f64().ok_or("invalid")? as f32,
        "max_step_height" => s.max_step_height = value.as_f64().ok_or("invalid")? as f32,
        "min_step_height" => s.min_step_height = value.as_f64().ok_or("invalid")? as f32,
        "min_duration" => s.min_duration = value.as_f64().ok_or("invalid")? as f32,
        "max_duration" => s.max_duration = value.as_f64().ok_or("invalid")? as f32,
        "min_button_interval_ms" => s.min_button_interval_ms = value.as_u64().ok_or("invalid")?,
        "step" => s.step = value.as_f64().ok_or("invalid")? as f32,
        "axis_smooth" => s.axis_smooth = value.as_f64().ok_or("invalid")? as f32,
        _ => return Err(format!("unknown key: {}", key)),
    }
    Ok(())
}

/// 获取最新一条发送的 command JSON 字符串
#[tauri::command]
pub fn get_latest_command(state: tauri::State<'_, Arc<RwLock<String>>>) -> String {
    state.read().unwrap().clone()
}

/// 获取最新系统监控状态（CPU、温度、电机等）
#[tauri::command]
pub fn get_system_status(
    state: tauri::State<'_, Arc<RwLock<Option<SystemStatus>>>>,
) -> Option<SystemStatus> {
    state.read().unwrap().clone()
}
