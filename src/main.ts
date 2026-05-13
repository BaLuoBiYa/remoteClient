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
});
