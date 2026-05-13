import { drainLogs, getLatestCommand } from "./backend";
import type { Settings } from "./backend";
import { getSettings, updateSetting, loadStoredSettings, saveSetting, WS_URL_FIRST_RUN_KEY, getSystemStatus } from "./backend";

// ========== 终端日志面板 ==========

const MAX_TERMINAL_LINES = 200;

export async function pollLogs(): Promise<void> {
    try {
        const lines = await drainLogs();
        if (lines.length === 0) return;
        const el = document.getElementById("terminal-output");
        if (!el) return;

        const current = el.textContent ?? "";
        const all = current.split("\n").filter(Boolean).concat(lines);
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

// ========== 系统监控：实时命令展示 ==========

let lastCmdText = "";

export async function pollCommand(): Promise<void> {
    try {
        const cmd = await getLatestCommand();
        if (!cmd || cmd === lastCmdText) return;
        lastCmdText = cmd;
        const el = document.getElementById("cmd-display");
        if (!el) return;
        // 格式化 JSON 展示
        try {
            const obj = JSON.parse(cmd);
            el.textContent = JSON.stringify(obj, null, 2);
        } catch {
            el.textContent = cmd;
        }
    } catch {
        // 忽略
    }
}

// ========== 参数调节面板 ==========

interface ParamDef {
    key: keyof Settings;
    label: string;
    min: number;
    max: number;
    step: number;
}

const PARAM_DEFS: ParamDef[] = [
    { key: "max_roll", label: "最大滚转角", min: 0, max: 1, step: 0.01 },
    { key: "max_pitch", label: "最大俯仰角", min: 0, max: 1, step: 0.01 },
    { key: "max_linear_speed", label: "最大线速度", min: 0, max: 1, step: 0.01 },
    { key: "exp_sensitivity", label: "指数灵敏度", min: 0.5, max: 10, step: 0.1 },
    { key: "max_base_height", label: "最大身高", min: 100, max: 500, step: 1 },
    { key: "min_base_height", label: "最小身高", min: 0, max: 200, step: 1 },
    { key: "max_step_height", label: "最大步高", min: 50, max: 300, step: 1 },
    { key: "min_step_height", label: "最小步高", min: 0, max: 100, step: 1 },
    { key: "min_duration", label: "最短步周期", min: 0.01, max: 0.5, step: 0.01 },
    { key: "max_duration", label: "最长步周期", min: 0.5, max: 3, step: 0.01 },
    { key: "min_button_interval_ms", label: "按键间隔(ms)", min: 50, max: 500, step: 10 },
    { key: "step", label: "调整步长", min: 1, max: 10, step: 0.1 },
    { key: "axis_smooth", label: "摇杆平滑", min: 0.05, max: 1.0, step: 0.05 },
];

export async function initParams(): Promise<void> {
    const container = document.getElementById("params-content");
    if (!container) return;

    let settings: Settings;
    try {
        settings = await getSettings();
    } catch {
        console.warn("获取设置失败，使用默认值");
        return;
    }

    // 本地存储覆盖后端默认
    const stored = loadStoredSettings();
    if (stored) {
        (Object.keys(stored) as (keyof Settings)[]).forEach((k) => {
            (settings as unknown as Record<string, unknown>)[k] = stored[k];
        });
        for (const [k, v] of Object.entries(stored)) {
            updateSetting(k, v).catch(console.error);
        }
    }

    const updateAndSave = (key: string, value: unknown) => {
        saveSetting(key as keyof Settings, value);
        updateSetting(key, value).catch(console.error);
    };

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
            updateAndSave(def.key, v);
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
            updateAndSave("curve", curveSelect.value);
        });
    }

    // WS URL 输入框
    const wsUrlParam = document.getElementById("ws-url-param") as HTMLInputElement;
    if (wsUrlParam) {
        wsUrlParam.value = settings.ws_url;
        wsUrlParam.addEventListener("change", () => {
            updateAndSave("ws_url", wsUrlParam.value);
        });
    }
}

// ========== 可拖拽分隔条 ==========

export function initSplitters(): void {
    const splitterH = document.getElementById("splitter-h")!;
    const leftPane = document.getElementById("left-pane")!;
    const rightPane = document.getElementById("right-pane")!;
    const splitterV = document.getElementById("splitter-v")!;
    const monitorPane = document.getElementById("monitor-pane")!;
    const paramsPane = document.getElementById("params-pane")!;

    restoreOrInitRatio("h-ratio", 0.66, leftPane, rightPane, "width");
    restoreOrInitVPanels(0.66, monitorPane, paramsPane);

    // 水平拖拽
    splitterH.addEventListener("mousedown", (e) => {
        e.preventDefault();
        splitterH.classList.add("active");
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";

        const layout = document.querySelector(".app-layout")!;
        const totalW = layout.clientWidth - 6;

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

    // 垂直拖拽
    splitterV.addEventListener("mousedown", (e) => {
        e.preventDefault();
        splitterV.classList.add("active");
        document.body.style.cursor = "row-resize";
        document.body.style.userSelect = "none";

        const totalH = leftPane.clientHeight - 6;

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
    key: string, defaultRatio: number,
    a: HTMLElement, b: HTMLElement, dim: "width" | "height"
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

function restoreOrInitVPanels(defaultRatio: number, monitor: HTMLElement, params: HTMLElement) {
    const leftPane = monitor.parentElement!;
    const stored = localStorage.getItem("v-ratio");
    const ratio = stored ? Math.max(0.12, Math.min(0.88, parseFloat(stored))) : defaultRatio;
    const total = leftPane.clientHeight - 6;
    const topH = Math.round(total * ratio);
    monitor.style.height = topH + "px";
    monitor.style.flex = "none";
    params.style.minHeight = "0";
}

// ========== WebSocket 配置弹窗 ==========

export function showWSModal(): Promise<string | null> {
    return new Promise((resolve) => {
        const modal = document.getElementById("ws-modal")!;
        const input = document.getElementById("ws-url-input") as HTMLInputElement;
        const confirmBtn = document.getElementById("ws-modal-confirm")!;
        const cancelBtn = document.getElementById("ws-modal-cancel")!;

        const stored = loadStoredSettings();
        if (stored?.ws_url) input.value = stored.ws_url;

        modal.style.display = "flex";
        input.focus();
        input.select();

        const cleanup = () => {
            confirmBtn.removeEventListener("click", onSubmit);
            cancelBtn.removeEventListener("click", onCancel);
            input.removeEventListener("keydown", onKey);
        };

        const onSubmit = () => {
            const url = input.value.trim();
            if (!url) return;
            modal.style.display = "none";
            saveSetting("ws_url", url);
            updateSetting("ws_url", url).catch(console.error);
            localStorage.setItem(WS_URL_FIRST_RUN_KEY, "1");
            cleanup();
            resolve(url);
            const wsUrlParam = document.getElementById("ws-url-param") as HTMLInputElement;
            if (wsUrlParam) wsUrlParam.value = url;
        };

        const onCancel = () => {
            modal.style.display = "none";
            cleanup();
            resolve(null);
        };

        const onKey = (e: KeyboardEvent) => {
            if (e.key === "Enter") onSubmit();
            if (e.key === "Escape") onCancel();
        };

        confirmBtn.addEventListener("click", onSubmit);
        cancelBtn.addEventListener("click", onCancel);
        input.addEventListener("keydown", onKey);
    });
}

// ========== 系统监控轮询 ==========

export async function pollSystemStatus(): Promise<void> {
    try {
        const status = await getSystemStatus();
        if (!status) return;

        // CPU 使用率
        const cpuEl = document.getElementById("cpu-usage");
        if (cpuEl) {
            cpuEl.textContent = status.cpu_usage.toFixed(1);
            cpuEl.className = `fw-bold ${status.cpu_usage > 80 ? "text-danger" : status.cpu_usage > 50 ? "text-warning" : ""}`;
        }

        // CPU 温度
        const tempEl = document.getElementById("cpu-temp");
        if (tempEl) {
            if (status.cpu_temperature < 0) {
                tempEl.textContent = "N/A";
                tempEl.className = "fw-bold text-muted";
            } else {
                tempEl.textContent = status.cpu_temperature.toFixed(1);
                tempEl.className = `fw-bold ${status.cpu_temperature > 80 ? "text-danger" : status.cpu_temperature > 60 ? "text-warning" : ""}`;
            }
        }

        // 控制频率
        const freqEl = document.getElementById("control-freq");
        if (freqEl) {
            freqEl.textContent = status.control_frequency.toFixed(1);
        }

        // 电机状态表
        const motorContainer = document.getElementById("motor-status");
        if (motorContainer && status.motors && status.motors.length > 0) {
            let html = "";
            for (const m of status.motors) {
                const tempColor = m.temperature > 70 ? "text-danger" : m.temperature > 50 ? "text-warning" : "";
                html += `<tr>
                    <td class="fw-bold text-center">${m.name}</td>
                    <td class="text-center">${m.position.toFixed(3)}</td>
                    <td class="text-center">${m.velocity.toFixed(2)}</td>
                    <td class="text-center">${m.torque.toFixed(2)}</td>
                    <td class="text-center ${tempColor}">${m.temperature}°C</td>
                </tr>`;
            }
            motorContainer.innerHTML = html;
        }
    } catch {
        // 忽略错误（后端可能尚未就绪）
    }
}
