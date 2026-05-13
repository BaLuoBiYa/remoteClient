import { invoke } from "@tauri-apps/api/core";

// ========== Settings 类型（与 Rust Settings struct 对齐） ==========
export interface Settings {
    ws_url: string;
    publish_topic: string;
    subscribe_topic: string;
    curve: "Linear" | "Exponential" | "SCurve";
    max_linear_speed_x: number;
    max_linear_speed_y: number;
    max_roll: number;
    max_pitch: number;
    exp_sensitivity: number;
    max_base_height: number;
    min_base_height: number;
    max_step_height: number;
    min_step_height: number;
    min_duration: number;
    max_duration: number;
    min_button_interval_ms: number;
    step: number;
    axis_smooth: number;
}

// ========== 系统监控类型（与 Rust SystemStatus/MotorState 对齐） ==========

export interface MotorState {
    name: string;
    position: number;
    velocity: number;
    torque: number;
    temperature: number;
}

export interface RosTime {
    sec: number;
    nanosec: number;
}

export interface SystemStatus {
    cpu_usage: number;
    cpu_temperature: number;
    motors: MotorState[];
    control_frequency: number;
    timestamp: RosTime;
}

// ========== 后端命令封装 ==========

/** 拉取并清空终端日志 */
export async function drainLogs(): Promise<string[]> {
    return invoke<string[]>("drain_logs");
}

/** 获取当前全部设置 */
export async function getSettings(): Promise<Settings> {
    return invoke<Settings>("get_settings");
}

/** 更新单个设置字段 */
export async function updateSetting(key: string, value: unknown): Promise<void> {
    await invoke("update_setting", { key, value });
}

/** 获取最新一条发送的 command JSON */
export async function getLatestCommand(): Promise<string> {
    return invoke<string>("get_latest_command");
}

/** 获取最新系统监控状态 */
export async function getSystemStatus(): Promise<SystemStatus | null> {
    return invoke<SystemStatus | null>("get_system_status");
}

// ========== 本地持久化 ==========

const STORAGE_KEY = "dog-settings";

export function loadStoredSettings(): Partial<Settings> | null {
    try {
        const raw = localStorage.getItem(STORAGE_KEY);
        return raw ? JSON.parse(raw) : null;
    } catch {
        return null;
    }
}

export function saveSetting(key: keyof Settings, value: unknown): void {
    try {
        const raw = localStorage.getItem(STORAGE_KEY);
        const obj: Record<string, unknown> = raw ? JSON.parse(raw) : {};
        obj[key] = value;
        localStorage.setItem(STORAGE_KEY, JSON.stringify(obj));
    } catch { /* ignore */ }
}

export function hasStoredUrl(): boolean {
    try {
        const raw = localStorage.getItem(STORAGE_KEY);
        if (!raw) return false;
        const obj = JSON.parse(raw);
        return typeof obj.ws_url === "string" && obj.ws_url.length > 0;
    } catch {
        return false;
    }
}

export const WS_URL_FIRST_RUN_KEY = "dog-ws-url-first-run";
