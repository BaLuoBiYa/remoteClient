import "bootstrap/dist/css/bootstrap.min.css";
import "bootstrap/dist/js/bootstrap.bundle.min.js";

import { hasStoredUrl, WS_URL_FIRST_RUN_KEY } from "./backend";
import { pollLogs, initParams, initSplitters, showWSModal, pollCommand, pollSystemStatus } from "./panels";

// ========== 入口 ==========
window.addEventListener("DOMContentLoaded", async () => {
    // 首次启动弹窗
    if (!hasStoredUrl() && !localStorage.getItem(WS_URL_FIRST_RUN_KEY)) {
        await showWSModal();
    }

    setInterval(pollLogs, 10);
    setInterval(pollCommand, 100);
    setInterval(pollSystemStatus, 200);
    initParams();
    initSplitters();

    // 重配按钮
    const reconfigBtn = document.getElementById("ws-reconfig-btn");
    if (reconfigBtn) {
        reconfigBtn.addEventListener("click", () => showWSModal());
    }

    // 重置按钮：清除所有本地存储
    const resetBtn = document.getElementById("ws-reset-btn");
    if (resetBtn) {
        resetBtn.addEventListener("click", () => {
            if (confirm("确认重置？将清除所有本地保存的参数（WS地址、话题、滑块值、窗口布局），恢复到初始状态。")) {
                localStorage.clear();
                location.reload();
            }
        });
    }
});
