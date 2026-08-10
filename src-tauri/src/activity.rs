use crate::state::{now_ms, Store};
use rdev::{listen, Event};
use std::sync::Arc;

/// 启动全局键鼠监听（独立线程，不阻塞主线程）
/// 隐私：rdev 仅回调事件类型/坐标，本函数只更新“最后活动时间”，绝不记录按键内容
pub fn start_listener(store: Arc<Store>) {
    std::thread::spawn(move || {
        // listen 会阻塞当前线程；回调中只取“发生了活动”这一事实
        if let Err(e) = listen(move |_event: Event| {
            let now = now_ms();
            if let Ok(mut s) = store.0.lock() {
                s.on_activity(now);
            }
        }) {
            eprintln!("[activity] 监听失败: {:?}", e);
        }
    });
}
