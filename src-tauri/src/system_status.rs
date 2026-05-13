use serde::{Deserialize, Serialize};

/// 单个电机实时状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorState {
    /// 电机名称 (LF_Front, RF_Rear, ...)
    pub name: String,
    /// 当前位置 (rad)
    pub position: f32,
    /// 当前速度 (rad/s)
    pub velocity: f32,
    /// 当前力矩 (N·m)
    pub torque: f32,
    /// 电机温度 (°C)
    pub temperature: i8,
}

/// 系统整体监控状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    /// CPU 使用率百分比 (0.0 ~ 100.0)
    pub cpu_usage: f32,
    /// CPU/SoC 温度 (°C)，-1 表示读取失败
    pub cpu_temperature: f32,
    /// 8 个电机状态
    pub motors: Vec<MotorState>,
    /// 主控制回路实时频率 (Hz)
    pub control_frequency: f32,
    /// 状态采集时间戳 (ROS time: {sec, nanosec})
    pub timestamp: RosTime,
}

/// ROS builtin_interfaces/Time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosTime {
    pub sec: i32,
    pub nanosec: u32,
}

/// rosbridge v2 发布消息的外层结构，用于反序列化
#[derive(Debug, Clone, Deserialize)]
pub struct RosbridgePublish {
    pub op: String,
    pub topic: Option<String>,
    pub msg: Option<serde_json::Value>,
}

impl SystemStatus {
    /// 尝试从 rosbridge v2 JSON 消息中解析 SystemStatus
    /// 期望格式: {"op":"publish", "topic":"/dog/monitor", "msg": {...}}
    pub fn try_from_rosbridge(json: &str) -> Option<Self> {
        let outer: RosbridgePublish = serde_json::from_str(json).ok()?;
        if outer.op != "publish" {
            return None;
        }
        let topic = outer.topic.as_deref().unwrap_or("");
        // 兼容带或不带前导 / 的 topic 名
        if topic != "/dog/monitor" && topic != "dog/monitor" {
            return None;
        }
        let msg = outer.msg?;
        serde_json::from_value(msg).ok()
    }
}
