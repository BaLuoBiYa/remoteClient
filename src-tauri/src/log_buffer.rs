use std::sync::LazyLock;
use std::sync::Mutex;

/// 最大保留日志行数
const MAX_LOG_LINES: usize = 1024;

/// 全局日志缓冲区，所有 `log_line!` 调用都写入这里
static LOG_BUFFER: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// 向全局日志缓冲区追加一行（带时间戳）
pub fn push_log(line: String) {
    let mut buf = LOG_BUFFER.lock().unwrap();
    let timestamp = chrono_now();
    buf.push(format!("[{}] {}", timestamp, line));
    // 超出上限时丢弃旧行
    if buf.len() > MAX_LOG_LINES {
        let excess = buf.len() - MAX_LOG_LINES;
        buf.drain(0..excess);
    }
}

/// 导出所有日志并清空缓冲区
pub fn drain_logs() -> Vec<String> {
    let mut buf = LOG_BUFFER.lock().unwrap();
    std::mem::take(&mut *buf)
}

/// 获取当前时间戳字符串（无需额外依赖 chrono）
fn chrono_now() -> String {
    // 使用 std::time + 简单的秒数，或者直接空时间戳
    // 也可以依赖 chrono，但避免引入新依赖，用 SystemTime
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    let millis = dur.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

/// 宏：写入一条日志到全局缓冲区
#[macro_export]
macro_rules! log_line {
    ($($arg:tt)*) => {
        $crate::log_buffer::push_log(format!($($arg)*))
    };
}
