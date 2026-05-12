import "bootstrap/dist/css/bootstrap.min.css";
import "bootstrap/dist/js/bootstrap.bundle.min.js";
import { invoke } from "@tauri-apps/api/core";

// ---------- 终端日志 ----------
const MAX_TERMINAL_LINES = 200;

async function pollLogs() {
  try {
    const lines: string[] = await invoke("drain_logs");
    if (lines.length === 0) return;
    const el = document.getElementById("terminal-output");
    if (!el) return;

    const current = el.textContent ?? "";
    const all = current
      .split("\n")
      .filter(Boolean)
      .concat(lines);
    if (all.length > MAX_TERMINAL_LINES) {
      all.splice(0, all.length - MAX_TERMINAL_LINES);
    }
    el.textContent = all.join("\n");

    const term = document.getElementById("terminal");
    if (term) term.scrollTop = term.scrollHeight;
  } catch (err) {
    console.error("拉取日志失败:", err);
  }
}

// ---------- 参数调节 ----------
interface Settings {
  curve: "Linear" | "Exponential" | "SCurve";
  max_linear_speed: number;
  max_angular: number;
  exp_sensitivity: number;
  max_base_height: number;
  min_base_height: number;
  max_step_height: number;
  min_step_height: number;
  min_duration: number;
  max_duration: number;
  min_button_interval_ms: number;
}

interface ParamDef {
  key: keyof Settings;
  label: string;
  min: number;
  max: number;
  step: number;
}

const PARAM_DEFS: ParamDef[] = [
  { key: "max_linear_speed", label: "最大线速度", min: 0, max: 1, step: 0.01 },
  { key: "max_angular", label: "最大角速度", min: 0, max: 1, step: 0.01 },
  { key: "exp_sensitivity", label: "指数灵敏度", min: 0.5, max: 10, step: 0.1 },
  { key: "max_base_height", label: "最大身高", min: 100, max: 500, step: 1 },
  { key: "min_base_height", label: "最小身高", min: 0, max: 200, step: 1 },
  { key: "max_step_height", label: "最大步高", min: 50, max: 300, step: 1 },
  { key: "min_step_height", label: "最小步高", min: 0, max: 100, step: 1 },
  { key: "min_duration", label: "最短步周期", min: 0.01, max: 0.5, step: 0.01 },
  { key: "max_duration", label: "最长步周期", min: 0.5, max: 3, step: 0.01 },
  { key: "min_button_interval_ms", label: "按键间隔(ms)", min: 50, max: 500, step: 10 },
];

async function initParams() {
  const container = document.getElementById("params-content");
  if (!container) return;

  let settings: Settings;
  try {
    settings = await invoke<Settings>("get_settings");
  } catch {
    console.warn("获取设置失败，使用默认值");
    return;
  }

  for (const def of PARAM_DEFS) {
    const val = settings[def.key];

    const row = document.createElement("div");
    row.className = "d-flex flex-column gap-1";

    const label = document.createElement("label");
    label.className = "d-flex justify-content-between small";
    label.innerHTML = `<span>${def.label}</span><span class="param-val">${val}</span>`;

    const range = document.createElement("input");
    range.type = "range";
    range.className = "form-range";
    range.min = String(def.min);
    range.max = String(def.max);
    range.step = String(def.step);
    range.value = String(val);

    range.addEventListener("input", () => {
      const v = parseFloat(range.value);
      const valSpan = label.querySelector(".param-val")!;
      valSpan.textContent = range.value;
      invoke("update_setting", { key: def.key, value: v }).catch(console.error);
    });

    row.appendChild(label);
    row.appendChild(range);
    container.appendChild(row);
  }

  // 曲线选择器
  const curveSelect = document.getElementById("curve-select") as HTMLSelectElement;
  if (curveSelect) {
    curveSelect.value = settings.curve;
    curveSelect.addEventListener("change", () => {
      invoke("update_setting", { key: "curve", value: curveSelect.value }).catch(console.error);
    });
  }
}

// ---------- 入口 ----------
window.addEventListener("DOMContentLoaded", () => {
  setInterval(pollLogs, 10);
  initParams();
  initSplitters();
});

// ---------- 拖拽分隔条 ----------
function initSplitters() {
  // 水平分隔条：调整 left / right 宽度
  const splitterH = document.getElementById("splitter-h")!;
  const leftPane = document.getElementById("left-pane")!;
  const rightPane = document.getElementById("right-pane")!;
  // 垂直分隔条：调整 monitor / params 高度
  const splitterV = document.getElementById("splitter-v")!;
  const monitorPane = document.getElementById("monitor-pane")!;
  const paramsPane = document.getElementById("params-pane")!;

  // 初始比例：左侧 66%，右侧 34%
  restoreOrInitRatio("h-ratio", 0.66, leftPane, rightPane, "width");
  // 初始比例：监控 66%，参数 34%
  restoreOrInitVPanels(0.66, monitorPane, paramsPane);

  // ---------- 水平拖拽 ----------
  splitterH.addEventListener("mousedown", (e) => {
    e.preventDefault();
    splitterH.classList.add("active");
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const layout = document.querySelector(".app-layout")!;
    const totalW = layout.clientWidth - 6; // 分隔条宽度

    const onMove = (ev: MouseEvent) => {
      const rect = layout.getBoundingClientRect();
      let leftW = ev.clientX - rect.left;
      leftW = Math.max(300, Math.min(totalW - 200, leftW));
      const rightW = totalW - leftW;
      leftPane.style.flex = "none";
      leftPane.style.width = leftW + "px";
      rightPane.style.flex = "none";
      rightPane.style.width = rightW + "px";
      localStorage.setItem("h-ratio", String(leftW / totalW));
    };

    const onUp = () => {
      splitterH.classList.remove("active");
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });

  // ---------- 垂直拖拽 ----------
  splitterV.addEventListener("mousedown", (e) => {
    e.preventDefault();
    splitterV.classList.add("active");
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";

    const totalH = leftPane.clientHeight - 6; // 分隔条高度

    const onMove = (ev: MouseEvent) => {
      const rect = leftPane.getBoundingClientRect();
      let topH = ev.clientY - rect.top;
      topH = Math.max(100, Math.min(totalH - 120, topH));
      monitorPane.style.height = topH + "px";
      monitorPane.style.flex = "none";
      paramsPane.style.minHeight = "0";
      localStorage.setItem("v-ratio", String(topH / totalH));
    };

    const onUp = () => {
      splitterV.classList.remove("active");
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });
}

function restoreOrInitRatio(
  key: string,
  defaultRatio: number,
  a: HTMLElement,
  b: HTMLElement,
  dim: "width" | "height"
) {
  const stored = localStorage.getItem(key);
  const ratio = stored ? Math.max(0.15, Math.min(0.85, parseFloat(stored))) : defaultRatio;
  const total = dim === "width"
    ? document.querySelector(".app-layout")!.clientWidth - 6
    : (a.parentElement ?? a).clientHeight - 6;
  const sizeA = Math.round(total * ratio);
  const sizeB = total - sizeA;
  a.style.flex = "none";
  a.style[dim] = sizeA + "px";
  b.style.flex = "none";
  b.style[dim] = sizeB + "px";
}

function restoreOrInitVPanels(
  defaultRatio: number,
  monitor: HTMLElement,
  params: HTMLElement
) {
  const leftPane = monitor.parentElement!;
  const stored = localStorage.getItem("v-ratio");
  const ratio = stored ? Math.max(0.12, Math.min(0.88, parseFloat(stored))) : defaultRatio;
  const total = leftPane.clientHeight - 6;
  const topH = Math.round(total * ratio);
  monitor.style.height = topH + "px";
  monitor.style.flex = "none";
  params.style.minHeight = "0";
}
