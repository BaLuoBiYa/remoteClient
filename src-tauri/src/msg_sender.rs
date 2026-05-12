use crate::log_line;
use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Notify;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// 默认 ROS2 桥接节点 WebSocket 地址
const DEFAULT_WS_URL: &str = "ws://NanoPi-M6.local:9090";

/// WebSocket 消息桥接器
///
/// 双队列架构：
/// - **发送队列**：外部 push → WS 后台任务自动取出并发送 → 发送后销毁
/// - **接收队列**：WS 后台任务收到消息 → push → 外部 drain 读取 → 读取后销毁
pub struct MsgBridge {
    /// 发送队列（外部写入，WS 任务消费）
    send_queue: Arc<StdMutex<Vec<String>>>,
    /// 接收队列（WS 任务写入，外部消费）
    recv_queue: Arc<StdMutex<Vec<String>>>,
    /// 通知 WS 任务有新消息待发送
    send_notify: Arc<Notify>,
    /// 连接状态
    #[allow(dead_code)]
    connected: Arc<StdMutex<bool>>,
}

impl MsgBridge {
    /// 创建 MsgBridge 并启动后台 WebSocket 连接任务（默认地址）
    pub fn new() -> Self {
        Self::with_url(DEFAULT_WS_URL)
    }

    /// 指定 URL 创建
    pub fn with_url(url: &str) -> Self {
        let send_queue = Arc::new(StdMutex::new(Vec::new()));
        let recv_queue = Arc::new(StdMutex::new(Vec::new()));
        let send_notify = Arc::new(Notify::new());
        let connected = Arc::new(StdMutex::new(false));

        let ws_url = url.to_string();
        let snd = send_queue.clone();
        let rcv = recv_queue.clone();
        let notify = send_notify.clone();
        let conn = connected.clone();

        // 在独立线程中启动 Tokio runtime 运行 WS 后台任务
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime for WS bridge");
            rt.block_on(async { ws_loop(&ws_url, snd, rcv, notify, conn).await });
        });

        Self {
            send_queue,
            recv_queue,
            send_notify,
            connected,
        }
    }

    /// 向发送队列 push 一条消息（同步，可在 Tauri 命令中调用）
    /// 消息交给 WS 后台任务自动发送，发送后从队列销毁
    pub fn push_send(&self, msg: String) {
        self.send_queue.lock().unwrap().push(msg);
        self.send_notify.notify_one();
    }

    /// 读取并清空接收队列（同步），返回所有已收到的消息
    /// 每条消息只被读取一次，读取后从队列销毁
    pub fn drain_recv(&self) -> Vec<String> {
        let mut queue = self.recv_queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }

    /// 获取连接状态
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }
}

/// WebSocket 连接维护循环
///
/// 职责：
/// 1. 维护 WS 连接，断线自动重连
/// 2. 从发送队列取消息 → 通过 WS 发出 → 销毁
/// 3. 从 WS 收消息 → 放入接收队列
async fn ws_loop(
    url: &str,
    send_queue: Arc<StdMutex<Vec<String>>>,
    recv_queue: Arc<StdMutex<Vec<String>>>,
    send_notify: Arc<Notify>,
    connected: Arc<StdMutex<bool>>,
) {
    loop {
        match connect_async(url).await {
            Ok((mut ws_stream, _)) => {
                *connected.lock().unwrap() = true;
                log_line!("[MsgBridge] WebSocket 已连接: {}", url);

                // 连接成功后立即发送队列中积压的消息
                drain_and_send(&send_queue, &mut ws_stream).await;

                // 主循环：同时处理发送和接收
                loop {
                    tokio::select! {
                        // 有新消息待发送
                        _ = send_notify.notified() => {
                            drain_and_send(&send_queue, &mut ws_stream).await;
                        }
                        // 收到 WS 消息 → 放入接收队列
                        msg = ws_stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    recv_queue.lock().unwrap().push(text.to_string());
                                }
                                Some(Ok(_)) => {} // 忽略非文本消息
                                Some(Err(e)) => {
                                    log_line!("[MsgBridge] WebSocket 接收错误: {}, 准备重连...", e);
                                    break;
                                }
                                None => {
                                    log_line!("[MsgBridge] WebSocket 连接关闭，准备重连...");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log_line!("[MsgBridge] WebSocket 连接失败: {}, 2秒后重试...", e);
            }
        }

        *connected.lock().unwrap() = false;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

/// 从发送队列取出所有消息并通过 WS 发出，发送后队列清空
async fn drain_and_send(
    send_queue: &Arc<StdMutex<Vec<String>>>,
    ws_stream: &mut (impl futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error>
              + Unpin),
) {
    let msgs: Vec<String> = {
        let mut queue = send_queue.lock().unwrap();
        std::mem::take(&mut *queue)
    };

    for msg in msgs {
        if let Err(e) = ws_stream.send(Message::Text(msg.into())).await {
            log_line!("[MsgBridge] WebSocket 发送失败: {}, 消息丢弃", e);
            // 发送失败时，消息已丢失（WS 连接状态异常），外层会重连
        }
    }
}
