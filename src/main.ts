import { invoke } from "@tauri-apps/api/core";
// ---------- 终端日志 ----------
const MAX_TERMINAL_LINES = 200;

async function pollLogs() {
  try {
    const lines: string[] = await invoke("drain_logs");
    if (lines.length === 0) return;
    const el = document.getElementById("terminal-output");
    if (!el) return;

    // 追加新行，超出上限裁旧
    const current = el.textContent ?? "";
    const all = current
      .split("\n")
      .filter(Boolean)
      .concat(lines);
    if (all.length > MAX_TERMINAL_LINES) {
      all.splice(0, all.length - MAX_TERMINAL_LINES);
    }
    el.textContent = all.join("\n");

    // 自动滚动到底部
    const term = document.getElementById("terminal");
    if (term) term.scrollTop = term.scrollHeight;
  } catch (err) {
    console.error("拉取日志失败:", err);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  // 日志轮询
  setInterval(pollLogs, 10);
});
