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
    /// 防止同一次按下反复切换：press 期间只允许 toggle 一次，
    /// 必须松开 LB/RB 后才允许下一次 toggle
    activate_toggle_lock: bool,

    // 摇杆低通滤波状态（EMA: filtered = α·raw + (1-α)·prev）
    filt_lx: f32,
    filt_ly: f32,
    filt_rx: f32,
    filt_ry: f32,
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
            activate_toggle_lock: false,
            filt_lx: 0.0,
            filt_ly: 0.0,
            filt_rx: 0.0,
            filt_ry: 0.0,
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
            // 如果本次 press 已触发过 toggle，不再计数，防止反复切换
            if self.activate_toggle_lock {
                return;
            }
            if self.activate_counter < 10 {
                self.activate_counter += 1;
            } else {
                self.activate = !self.activate;
                self.activate_counter = 0;
                self.activate_toggle_lock = true; // 本次 press 不再允许 toggle
            }
        } else {
            self.activate_counter = 0;
            self.activate_toggle_lock = false; // 松开后解除，允许下一次 toggle
        }
    }

    fn map_axes(&mut self, s: &GamepadState, cfg: &crate::Settings) {
        let alpha = cfg.axis_smooth.clamp(0.0, 1.0);

        // EMA 低通滤波
        self.filt_lx = alpha * s.left_stick_x + (1.0 - alpha) * self.filt_lx;
        self.filt_ly = alpha * s.left_stick_y + (1.0 - alpha) * self.filt_ly;
        self.filt_rx = alpha * s.right_stick_x + (1.0 - alpha) * self.filt_rx;
        self.filt_ry = alpha * s.right_stick_y + (1.0 - alpha) * self.filt_ry;

        if self.filt_lx.abs() < 0.05 {
            self.filt_lx = 0.0;
        }
        if self.filt_ly.abs() < 0.05 {
            self.filt_ly = 0.0;
        }
        if self.filt_rx.abs() < 0.05 {
            self.filt_rx = 0.0;
        }
        if self.filt_ry.abs() < 0.05 {
            self.filt_ry = 0.0;
        }

        let c = cfg.curve;
        self.cmd.linear_x = c.map(self.filt_ly, cfg.max_linear_speed, cfg.exp_sensitivity);
        self.cmd.linear_y = -1.0 * c.map(self.filt_lx, cfg.max_linear_speed, cfg.exp_sensitivity);
        self.cmd.roll = c.map(self.filt_rx, cfg.max_roll, cfg.exp_sensitivity);
        self.cmd.pitch = c.map(self.filt_ry, cfg.max_pitch, cfg.exp_sensitivity);
    }

    fn map_gait(&mut self, s: &GamepadState) {
        if s.btn_west {
            // self.cmd.gait = Gait::DEACTIVE;
        } else if s.btn_east {
            self.cmd.gait = Gait::WALK;
        } else if s.btn_north {
            // self.cmd.gait = Gait::JUMP;
        } else if s.btn_south {
            self.cmd.gait = Gait::STAND;
        }
    }

    fn map_increments(&mut self, s: &GamepadState, cfg: &crate::Settings) {
        let min_interval = Duration::from_millis(cfg.min_button_interval_ms);

        if s.left_button {
            if self.last_step_height.elapsed() >= min_interval {
                self.cmd.step_height -= cfg.step;
                self.last_step_height = Instant::now();
            }
        } else if s.right_button {
            if self.last_step_height.elapsed() >= min_interval {
                self.cmd.step_height += cfg.step;
                self.last_step_height = Instant::now();
            }
        }

        if s.btn_dpad_left {
            if self.last_step_duration.elapsed() >= min_interval {
                self.cmd.step_duration -= cfg.step / 100.0;
                self.last_step_duration = Instant::now();
            }
        } else if s.btn_dpad_right {
            if self.last_step_duration.elapsed() >= min_interval {
                self.cmd.step_duration += cfg.step / 100.0;
                self.last_step_duration = Instant::now();
            }
        }

        if s.btn_dpad_down {
            if self.last_base_height.elapsed() >= min_interval {
                self.cmd.base_height -= cfg.step;
                self.last_base_height = Instant::now();
            }
        } else if s.btn_dpad_up {
            if self.last_base_height.elapsed() >= min_interval {
                self.cmd.base_height += cfg.step;
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
