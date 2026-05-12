/// 步态枚举 — 与 ROS2 自定义消息 gait 字段对齐
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gait {
    STAND = 0,
    WALK = 1,
    JUMP = 2,
    DEACTIVE = 3,
}

pub struct DogCommand {
    pub gait: Gait,

    pub step_height: f32,
    pub base_height: f32,
    pub step_duration: f32,

    pub linear_x: f32,
    pub linear_y: f32,

    pub roll: f32,
    pub pitch: f32,

    topic: String,
}

fn round_f(v: f32, digits: i32) -> f64 {
    let scale = 10f64.powi(digits);
    (v as f64 * scale).round() / scale
}

impl DogCommand {
    pub fn new(topic: String) -> Self {
        Self {
            gait: Gait::STAND,
            step_height: 30.0,
            base_height: 200.0,
            step_duration: 0.5,
            linear_x: 0.0,
            linear_y: 0.0,
            roll: 0.0,
            pitch: 0.0,
            topic,
        }
    }

    /// 序列化为 rosbridge v2 publish JSON 字符串
    pub fn build(&self) -> String {
        let msg = serde_json::json!({
            "op": "publish",
            "topic": self.topic,
            "msg": {
                "gait": self.gait as u8,
                "step_height": round_f(self.step_height, 3),
                "base_height": round_f(self.base_height, 3),
                "step_duration": round_f(self.step_duration, 3),
                "linear_x": round_f(self.linear_x, 3),
                "linear_y": round_f(self.linear_y, 3),
                "roll": round_f(self.roll, 3),
                "pitch": round_f(self.pitch, 3),
            }
        });
        msg.to_string()
    }
}
