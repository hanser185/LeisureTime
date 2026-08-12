use crate::state::{now_ms, Store};
use rdev::{listen, Event};
use std::sync::Arc;

/// 启动全局键鼠监听（独立线程，不阻塞主线程）
/// 隐私：rdev 仅回调事件类型/坐标，本函数只更新“最后活动时间”，绝不记录按键内容
pub fn start_listener(store: Arc<Store>) {
    std::thread::spawn(move || loop {
        // listen 会阻塞当前线程；回调中只取“发生了活动”这一事实
        let s = Arc::clone(&store);
        if let Err(e) = listen(move |_event: Event| {
            let now = now_ms();
            if let Ok(mut st) = s.0.lock() {
                st.on_activity(now);
            }
        }) {
            // 监听异常退出（如权限/驱动问题）：日志提示并退避重试，避免活动检测静默失效
            eprintln!("[activity] 监听失败，5s 后重试: {:?}", e);
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            // 极少见的正常返回，保持线程存活
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}
