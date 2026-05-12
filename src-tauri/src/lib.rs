mod command_bulider;
mod game_pad;
mod log_buffer;
mod msg_sender;

use command_bulider::{DogCommand, Gait};
use game_pad::GamepadInterface;
use msg_sender::MsgBridge;
use std::thread;

const MAX_LINER_SPEED: f32 = 0.1;
const MAX_ANGULAR: f32 = 0.1;
/// 指数曲线灵敏度：值越大，中心死区越细腻，边缘越陡
const EXP_SENSITIVITY: f32 = 3.0;

const MAX_BASE_HEIGHT: f32 = 300.0;
const MIN_BASE_HEIGHT: f32 = 80.0;

const MAX_STEP_HEIGHT: f32 = 150.0;
const MIN_STEP_HEIGHT: f32 = 20.0;

const MIN_DURATION: f32 = 0.1;
const MAX_DURATION: f32 = 1.0;

// 按键最小触发间隔（毫秒），防止一次按下在多次循环中被多次计数
const MIN_BUTTON_INTERVAL_MS: u64 = 150;

/// 指数曲线映射：v = sign(x) * (e^(k·|x|) − 1) / (e^k − 1) * max_out
fn exp_map(input: f32, max_out: f32) -> f32 {
    let k = EXP_SENSITIVITY;
    let abs = input.abs().clamp(0.0, 1.0);
    let scale = if abs > 0.0 {
        (f32::exp_m1(k * abs)) / (f32::exp_m1(k))
    } else {
        0.0
    };
    input.signum() * scale * max_out
}

fn round_to(x: f32, digits: i32) -> f32 {
    let scale = 10f32.powi(digits);
    (x * scale).round() / scale
}

/// 读取并清空日志缓冲区，返回所有后端日志行（读取后销毁）
#[tauri::command]
fn drain_logs() -> Vec<String> {
    log_buffer::drain_logs()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ---------- 启动手柄后台读取线程 ----------
    // 该线程独立运行 Tokio runtime，循环读取手柄状态并推入日志
    let _gamepad_handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("failed to build tokio runtime for gamepad");

        rt.block_on(async {
            let mut gamepad =
                GamepadInterface::new().expect("failed to initialize gamepad interface");

            let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
            let mut command = DogCommand::new("/dog/command".to_string());

            let mut activate: bool = false;
            let mut activate_counter: u8 = 0;

            let ws_bridge = MsgBridge::new();

            // 防抖：记录每个可变参数上次被修改的时间
            let min_interval = std::time::Duration::from_millis(MIN_BUTTON_INTERVAL_MS);
            let mut last_step_height = std::time::Instant::now() - min_interval;
            let mut last_step_duration = std::time::Instant::now() - min_interval;
            let mut last_base_height = std::time::Instant::now() - min_interval;

            loop {
                interval.tick().await;
                gamepad.update();
                let s = gamepad.state();
                if !s.connected {
                    continue; // 跳过未连接状态
                }

                if s.left_button && s.right_button {
                    if activate_counter < 10 {
                        activate_counter += 1;
                        continue;
                    } else {
                        if activate {
                            activate = false;
                            // command.gait = Gait::STAND;
                            // command.base_height = 170.0;
                            // ws_bridge.push_send(command.build());
                        } else {
                            activate = true;
                        }
                        activate_counter = 0;
                    }
                } else {
                    activate_counter = 0;
                }
                // 连续10周期按下解锁

                if activate {
                    command.linear_x = exp_map(s.left_stick_y, MAX_LINER_SPEED);
                    command.linear_y = exp_map(s.left_stick_x, MAX_LINER_SPEED);
                    command.roll = exp_map(s.right_stick_x, MAX_ANGULAR);
                    command.pitch = exp_map(s.right_stick_y, MAX_ANGULAR);

                    if s.btn_west {
                        command.gait = Gait::STAND;
                    } else if s.btn_east {
                        command.gait = Gait::WALK;
                    } else if s.btn_north {
                        command.gait = Gait::JUMP;
                    }

                    if s.left_button {
                        if last_step_height.elapsed() >= min_interval {
                            command.step_height -= 10.0;
                            last_step_height = std::time::Instant::now();
                        }
                    } else if s.right_button {
                        if last_step_height.elapsed() >= min_interval {
                            command.step_height += 10.0;
                            last_step_height = std::time::Instant::now();
                        }
                    }

                    if s.btn_dpad_left {
                        if last_step_duration.elapsed() >= min_interval {
                            command.step_duration -= 0.01;
                            last_step_duration = std::time::Instant::now();
                        }
                    } else if s.btn_dpad_right {
                        if last_step_duration.elapsed() >= min_interval {
                            command.step_duration += 0.01;
                            last_step_duration = std::time::Instant::now();
                        }
                    }

                    if s.btn_dpad_down {
                        if last_base_height.elapsed() >= min_interval {
                            command.base_height -= 10.0;
                            last_base_height = std::time::Instant::now();
                        }
                    } else if s.btn_dpad_up {
                        if last_base_height.elapsed() >= min_interval {
                            command.base_height += 10.0;
                            last_base_height = std::time::Instant::now();
                        }
                    }

                    command.base_height =
                        command.base_height.clamp(MIN_BASE_HEIGHT, MAX_BASE_HEIGHT);
                    command.step_height =
                        command.step_height.clamp(MIN_STEP_HEIGHT, MAX_STEP_HEIGHT);
                    command.step_duration = command.step_duration.clamp(MIN_DURATION, MAX_DURATION);

                    command.linear_x = round_to(command.linear_x, 3);
                    command.linear_y = round_to(command.linear_y, 3);
                    command.roll = round_to(command.roll, 3);
                    command.pitch = round_to(command.pitch, 3);
                    command.step_height = round_to(command.step_height, 3);
                    command.step_duration = round_to(command.step_duration, 3);
                    command.base_height = round_to(command.base_height, 3);

                    log_line!("{}", command.build());
                    ws_bridge.push_send(command.build());
                }
            }
        });
    });

    // ---------- Tauri 启动 ----------
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![drain_logs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
