use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use serde::Serialize;

/// 手柄状态 —— 暴露给外部用户的纯数据结构
#[derive(Debug, Clone, Serialize)]
pub struct GamepadState {
    pub connected: bool,
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,

    pub left_trigger: f32,
    pub right_trigger: f32,
    pub left_button: bool,
    pub right_button: bool,

    pub btn_south: bool,
    pub btn_east: bool,
    pub btn_north: bool,
    pub btn_west: bool,

    pub btn_dpad_up: bool,
    pub btn_dpad_down: bool,
    pub btn_dpad_left: bool,
    pub btn_dpad_right: bool,
}

/// 手柄接口：内部实现完全隐藏，只暴露 GamepadState
pub struct GamepadInterface {
    gilrs: Gilrs,
    active_id: Option<GamepadId>,
    state: GamepadState, // 内部维护的状态
}

impl GamepadInterface {
    /// 构造函数
    pub fn new() -> Result<Self, gilrs::Error> {
        let gilrs = Gilrs::new()?;
        let active_id = gilrs.gamepads().next().map(|(id, _)| id);

        let mut iface = Self {
            gilrs,
            active_id,
            state: GamepadState::default(),
        };
        iface.refresh_state(); // 初始化时填充一次状态
        Ok(iface)
    }

    /// 外部唯一调用的方法：刷新手柄状态
    pub fn update(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    if self.active_id.is_none() {
                        self.active_id = Some(event.id);
                    }
                }
                EventType::Disconnected => {
                    if Some(event.id) == self.active_id {
                        self.active_id = self.gilrs.gamepads().next().map(|(id, _)| id);
                    }
                }
                _ => {}
            }
        }
        self.refresh_state();
    }

    /// 获取当前状态（只读引用）
    pub fn state(&self) -> &GamepadState {
        &self.state
    }

    // ---------- 私有方法 ----------
    fn refresh_state(&mut self) {
        if let Some(id) = self.active_id {
            let pad = self.gilrs.gamepad(id);
            self.state.connected = pad.is_connected();
            self.state.left_stick_x = pad.value(Axis::LeftStickX);
            self.state.left_stick_y = pad.value(Axis::LeftStickY);
            self.state.right_stick_x = pad.value(Axis::RightStickX);
            self.state.right_stick_y = pad.value(Axis::RightStickY);

            let left_trigger_data = pad.button_data(Button::LeftTrigger2);
            let right_trigger_data = pad.button_data(Button::RightTrigger2);
            self.state.left_trigger = left_trigger_data.map_or(0.0, |d| d.value());
            self.state.right_trigger = right_trigger_data.map_or(0.0, |d| d.value());

            self.state.left_button = pad.is_pressed(Button::LeftTrigger);
            self.state.right_button = pad.is_pressed(Button::RightTrigger);

            self.state.btn_south = pad.is_pressed(Button::South);
            self.state.btn_east = pad.is_pressed(Button::East);
            self.state.btn_north = pad.is_pressed(Button::North);
            self.state.btn_west = pad.is_pressed(Button::West);

            self.state.btn_dpad_up = pad.is_pressed(Button::DPadUp);
            self.state.btn_dpad_down = pad.is_pressed(Button::DPadDown);
            self.state.btn_dpad_left = pad.is_pressed(Button::DPadLeft);
            self.state.btn_dpad_right = pad.is_pressed(Button::DPadRight);
        } else {
            self.state = GamepadState::default();
        }
    }
}

/// 默认状态：全部归零
impl Default for GamepadState {
    fn default() -> Self {
        Self {
            connected: false,
            left_stick_x: 0.0,
            left_stick_y: 0.0,
            right_stick_x: 0.0,
            right_stick_y: 0.0,

            left_trigger: 0.0,
            right_trigger: 0.0,
            left_button: false,
            right_button: false,

            btn_south: false,
            btn_east: false,
            btn_north: false,
            btn_west: false,

            btn_dpad_up: false,
            btn_dpad_down: false,
            btn_dpad_left: false,
            btn_dpad_right: false,
        }
    }
}
