use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::command_bulider::{DogCommand, Gait};
use crate::game_pad::GamepadState;

// ========== 映射曲线 ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Curve {
    Linear,
    Exponential,
    SCurve,
}

impl Curve {
    /// 将 [-1.0, 1.0] 摇杆输入映射到 [-max_out, max_out]
    pub fn map(&self, x: f32, max_out: f32, sensitivity: f32) -> f32 {
        match self {
            Curve::Linear => linear_map(x, max_out),
            Curve::Exponential => exp_map(x, max_out, sensitivity),
            Curve::SCurve => scurve_map(x, max_out, sensitivity),
        }
    }
}

/// 线性：v = x * max_out
fn linear_map(x: f32, max_out: f32) -> f32 {
    x.clamp(-1.0, 1.0) * max_out
}

/// 指数曲线：v = sign(x) * (e^(k·|x|) − 1) / (e^k − 1) * max_out
fn exp_map(x: f32, max_out: f32, sensitivity: f32) -> f32 {
    let k = sensitivity;
    let abs = x.abs().clamp(0.0, 1.0);
    let scale = if abs > 0.0 {
        (f32::exp_m1(k * abs)) / (f32::exp_m1(k))
    } else {
        0.0
    };
    x.signum() * scale * max_out
}

/// S 曲线（归一化 Sigmoid）：
///   raw    = 2/(1+e^(-k·x)) − 1          ← 值域 (-1,1)
///   norm   = 2/(1+e^(-k))   − 1          ← x=1 处的 raw 值
///   output = raw / norm * max_out
fn scurve_map(x: f32, max_out: f32, sensitivity: f32) -> f32 {
    let k = sensitivity;
    let raw = 2.0 / (1.0 + f32::exp(-k * x)) - 1.0;
    let norm = 2.0 / (1.0 + f32::exp(-k)) - 1.0;
    if norm.abs() < 1e-6 {
        return 0.0;
    }
    (raw / norm) * max_out
}

// ========== 映射器 ==========

/// 手柄状态 → DogCommand 的完整映射器
/// 封装：摇杆曲线映射、按键步态切换、增量参数调节
pub struct Mapper {
    pub cmd: DogCommand,

    // 防抖计时
    last_step_height: Instant,
    last_step_duration: Instant,
    last_base_height: Instant,

    // 激活锁计数器（LB+RB 长按解锁）
    pub activate: bool,
    activate_counter: u8,
}

impl Mapper {
    pub fn new(topic: String) -> Self {
        let now = Instant::now();
        Self {
            cmd: DogCommand::new(topic),
            last_step_height: now,
            last_step_duration: now,
            last_base_height: now,
            activate: false,
            activate_counter: 0,
        }
    }

    /// 每周期调用一次：根据手柄状态和设置更新内部指令
    /// 返回是否需要发送指令
    pub fn apply(&mut self, s: &GamepadState, cfg: &crate::Settings) -> bool {
        self.update_activate(s);
        if !self.activate {
            return false;
        }

        self.map_axes(s, cfg);
        self.map_gait(s);
        self.map_increments(s, cfg);
        self.clamp_all(cfg);
        true
    }

    // ---- 私有 ----

    fn update_activate(&mut self, s: &GamepadState) {
        if s.left_button && s.right_button {
            if self.activate_counter < 10 {
                self.activate_counter += 1;
            } else {
                self.activate = !self.activate;
                self.activate_counter = 0;
            }
        } else {
            self.activate_counter = 0;
        }
    }

    fn map_axes(&mut self, s: &GamepadState, cfg: &crate::Settings) {
        let c = cfg.curve;
        self.cmd.linear_x = c.map(s.left_stick_y, cfg.max_linear_speed, cfg.exp_sensitivity);
        self.cmd.linear_y = c.map(s.left_stick_x, cfg.max_linear_speed, cfg.exp_sensitivity);
        self.cmd.roll = c.map(s.right_stick_x, cfg.max_angular, cfg.exp_sensitivity);
        self.cmd.pitch = c.map(s.right_stick_y, cfg.max_angular, cfg.exp_sensitivity);
    }

    fn map_gait(&mut self, s: &GamepadState) {
        if s.btn_west {
            self.cmd.gait = Gait::DEACTIVE;
        } else if s.btn_east {
            self.cmd.gait = Gait::WALK;
        } else if s.btn_north {
            self.cmd.gait = Gait::JUMP;
        } else if s.btn_south {
            self.cmd.gait = Gait::STAND;
        }
    }

    fn map_increments(&mut self, s: &GamepadState, cfg: &crate::Settings) {
        let min_interval = Duration::from_millis(cfg.min_button_interval_ms);

        if s.left_button {
            if self.last_step_height.elapsed() >= min_interval {
                self.cmd.step_height -= 10.0;
                self.last_step_height = Instant::now();
            }
        } else if s.right_button {
            if self.last_step_height.elapsed() >= min_interval {
                self.cmd.step_height += 10.0;
                self.last_step_height = Instant::now();
            }
        }

        if s.btn_dpad_left {
            if self.last_step_duration.elapsed() >= min_interval {
                self.cmd.step_duration -= 0.01;
                self.last_step_duration = Instant::now();
            }
        } else if s.btn_dpad_right {
            if self.last_step_duration.elapsed() >= min_interval {
                self.cmd.step_duration += 0.01;
                self.last_step_duration = Instant::now();
            }
        }

        if s.btn_dpad_down {
            if self.last_base_height.elapsed() >= min_interval {
                self.cmd.base_height -= 10.0;
                self.last_base_height = Instant::now();
            }
        } else if s.btn_dpad_up {
            if self.last_base_height.elapsed() >= min_interval {
                self.cmd.base_height += 10.0;
                self.last_base_height = Instant::now();
            }
        }
    }

    fn clamp_all(&mut self, cfg: &crate::Settings) {
        self.cmd.base_height = self
            .cmd
            .base_height
            .clamp(cfg.min_base_height, cfg.max_base_height);
        self.cmd.step_height = self
            .cmd
            .step_height
            .clamp(cfg.min_step_height, cfg.max_step_height);
        self.cmd.step_duration = self
            .cmd
            .step_duration
            .clamp(cfg.min_duration, cfg.max_duration);
    }
}
