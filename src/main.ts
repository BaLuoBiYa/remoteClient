import { invoke } from "@tauri-apps/api/core";

// ---------- greet 功能 ----------
let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;

async function greet() {
  if (greetMsgEl && greetInputEl) {
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
}

// ---------- 手柄状态 ----------
interface GamepadState {
  connected: boolean;

  left_stick_x: number;
  left_stick_y: number;
  right_stick_x: number;
  right_stick_y: number;

  left_trigger: number;
  right_trigger: number;

  left_button: boolean;
  right_button: boolean;

  btn_south: boolean;
  btn_east: boolean;
  btn_north: boolean;
  btn_west: boolean;

  btn_dpad_up: boolean;
  btn_dpad_down: boolean;
  btn_dpad_left: boolean;
  btn_dpad_right: boolean;
}

/** 更新单个数值元素，保留两位小数 */
function setText(id: string, value: number, fraction = 2) {
  const el = document.getElementById(id);
  if (el) el.textContent = value.toFixed(fraction);
}

/** 更新布尔指示灯 */
function setBadge(id: string, active: boolean) {
  const el = document.getElementById(id);
  if (!el) return;
  if (active) {
    el.textContent = "●";
    el.className = "btn-on";
  } else {
    el.textContent = "○";
    el.className = "btn-off";
  }
}

async function pollGamepad() {
  try {
    const state: GamepadState = await invoke("get_gamepad_state");

    // 连接状态
    const connEl = document.getElementById("gp-connected");
    if (connEl) {
      connEl.textContent = state.connected ? "已连接" : "未连接";
      connEl.className = state.connected
        ? "status-badge connected"
        : "status-badge disconnected";
    }

    // 摇杆
    setText("gp-ls-x", state.left_stick_x);
    setText("gp-ls-y", state.left_stick_y);
    setText("gp-rs-x", state.right_stick_x);
    setText("gp-rs-y", state.right_stick_y);

    // 扳机
    setText("gp-lt", state.left_trigger);
    setText("gp-rt", state.right_trigger);

    // 按键
    setBadge("gp-lb", state.left_button);
    setBadge("gp-rb", state.right_button);
    setBadge("gp-south", state.btn_south);
    setBadge("gp-east", state.btn_east);
    setBadge("gp-west", state.btn_west);
    setBadge("gp-north", state.btn_north);
    setBadge("gp-dpad-up", state.btn_dpad_up);
    setBadge("gp-dpad-down", state.btn_dpad_down);
    setBadge("gp-dpad-left", state.btn_dpad_left);
    setBadge("gp-dpad-right", state.btn_dpad_right);
  } catch (err) {
    console.error("获取手柄状态失败:", err);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  // greet
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  document.querySelector("#greet-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  // 手柄轮询，约 60 Hz
  setInterval(pollGamepad, 16);
});
