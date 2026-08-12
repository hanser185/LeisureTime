use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 用户状态：空闲 / 工作中 / 休息中
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    Idle,
    Working,
    Resting,
}

/// 单个工作/休息片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_sec: u64,
}

/// 当日数据（按日落盘）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DailyData {
    pub date: String,
    pub work_segments: Vec<Segment>,
    pub rest_segments: Vec<Segment>,
    pub water_intakes: Vec<String>, // "09:15"
    pub rest_count: u32,
    pub rest_reminders: u32,
    pub last_activity_ms: u64,
    pub last_water_prompt_ms: u64,
}

/// 用户设置（独立文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub work_threshold_min: u64, // 默认 60
    pub rest_threshold_min: u64, // 默认 10
    pub reminder_mode: String,   // "toast" | "popup" | "fullscreen"
    pub water_enabled: bool,     // 默认 true
    pub water_interval_min: u64, // 默认 60
    pub work_hours_only: bool,   // 默认 false
    pub work_start: String,      // "09:00"
    pub work_end: String,        // "18:00"
    pub paused: bool,            // 默认 false
    pub autostart: bool,         // 默认 false
    pub theme: String,           // "light" | "dark" | "system"
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            work_threshold_min: 60,
            rest_threshold_min: 10,
            reminder_mode: "toast".into(),
            water_enabled: true,
            water_interval_min: 60,
            work_hours_only: false,
            work_start: "09:00".into(),
            work_end: "18:00".into(),
            paused: false,
            autostart: false,
            theme: "system".into(),
        }
    }
}

/// 全局可变状态（被 Mutex 保护，跨线程共享）
pub struct AppState {
    pub user_state: UserState,
    pub daily: DailyData,
    pub settings: Settings,
    pub segment_start_ms: u64,
    pub rest_fired_for_segment: bool,
    pub snooze_until_ms: u64,
    pub rest_segment_start_ms: u64,
    pub current_date: String,
    pub water_deferred: bool, // 休息提醒在场时，喝水提醒延后
    pub last_save_ms: u64,
}

impl AppState {
    pub fn new(settings: Settings, mut daily: DailyData, date: String) -> Self {
        let now = now_ms();
        // 启动即视为全新片段：不沿用历史 last_activity_ms 作为活动起点，
        // 否则从磁盘读到的旧时间戳会让首个 tick 误判“已连续工作很久”而立即弹休息提醒；
        // 同时以当前时间为喝水提醒基准，避免首次工作就立即弹喝水提醒。
        daily.last_activity_ms = now;
        daily.last_water_prompt_ms = now;
        AppState {
            user_state: UserState::Idle,
            daily,
            settings,
            segment_start_ms: 0,
            rest_fired_for_segment: false,
            snooze_until_ms: 0,
            rest_segment_start_ms: 0,
            current_date: date,
            water_deferred: false,
            last_save_ms: now,
        }
    }

    /// 记录一次活动（由 rdev 回调调用，不读取任何按键内容）
    pub fn on_activity(&mut self, now: u64) {
        if self.settings.paused {
            return;
        }
        match self.user_state {
            UserState::Idle => {
                self.user_state = UserState::Working;
                self.segment_start_ms = now;
            }
            UserState::Resting => {
                self.close_rest_segment(now);
                self.user_state = UserState::Working;
                self.segment_start_ms = now;
                self.rest_fired_for_segment = false;
            }
            UserState::Working => {}
        }
        self.daily.last_activity_ms = now;
    }

    /// 每秒 tick：推进状态机；跨日返回需保存的旧日数据
    pub fn tick(&mut self, now: u64) -> Option<DailyData> {
        if self.settings.paused {
            return None;
        }
        let today = today_string();
        if today != self.current_date {
            // 关闭当前未闭合片段并按日归档
            self.flush();
            let old = std::mem::replace(
                &mut self.daily,
                DailyData {
                    date: today.clone(),
                    ..Default::default()
                },
            );
            self.current_date = today;
            self.user_state = UserState::Idle;
            self.segment_start_ms = 0;
            self.rest_segment_start_ms = 0;
            self.rest_fired_for_segment = false;
            self.snooze_until_ms = 0;
            self.water_deferred = false;
            self.last_save_ms = now;
            return Some(old);
        }

        let rest_th = self.settings.rest_threshold_min * 60_000;
        let since = now.saturating_sub(self.daily.last_activity_ms);
        match self.user_state {
            UserState::Idle => {}
            UserState::Working => {
                if self.daily.last_activity_ms > 0 && since >= rest_th {
                    self.close_work_segment(now);
                    self.user_state = UserState::Resting;
                    self.rest_segment_start_ms = now;
                    self.daily.rest_count += 1;
                }
            }
            UserState::Resting => {
                if since < rest_th {
                    self.close_rest_segment(now);
                    self.user_state = UserState::Working;
                    self.segment_start_ms = now;
                    self.rest_fired_for_segment = false;
                }
            }
        }
        None
    }

    pub fn close_work_segment(&mut self, now: u64) {
        if self.segment_start_ms == 0 {
            return;
        }
        let dur = (now.saturating_sub(self.segment_start_ms)) / 1000;
        if dur > 0 {
            self.daily.work_segments.push(Segment {
                start_ms: self.segment_start_ms,
                end_ms: now,
                duration_sec: dur,
            });
        }
        self.segment_start_ms = 0;
    }

    pub fn close_rest_segment(&mut self, now: u64) {
        if self.rest_segment_start_ms == 0 {
            return;
        }
        let dur = (now.saturating_sub(self.rest_segment_start_ms)) / 1000;
        if dur > 0 {
            self.daily.rest_segments.push(Segment {
                start_ms: self.rest_segment_start_ms,
                end_ms: now,
                duration_sec: dur,
            });
        }
        self.rest_segment_start_ms = 0;
    }

    /// 当前工作片段已连续时长（毫秒）
    pub fn current_segment_ms(&self, now: u64) -> u64 {
        if self.user_state == UserState::Working && self.segment_start_ms > 0 {
            now.saturating_sub(self.segment_start_ms)
        } else {
            0
        }
    }

    /// 退出时保存当前未闭合片段，返回需落盘的当日数据
    pub fn flush(&mut self) {
        let now = now_ms();
        if self.user_state == UserState::Working {
            self.close_work_segment(now);
        } else if self.user_state == UserState::Resting {
            self.close_rest_segment(now);
        }
    }
}

/// 供 Tauri 托管的包装
pub struct Store(pub Mutex<AppState>);

impl Store {
    pub fn new(settings: Settings, daily: DailyData, date: String) -> Self {
        Store(Mutex::new(AppState::new(settings, daily, date)))
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn today_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn now_hm() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

/// 将 "HH:MM" 解析为当天分钟数（0–1439），非法返回 None
fn parse_hm(s: &str) -> Option<u16> {
    let mut it = s.split(':');
    let h: u16 = it.next()?.parse().ok()?;
    let m: u16 = it.next()?.parse().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}

/// 纯函数：给定“是否仅工作时段”“起止”“当前分钟”，判定是否处于工作时段。
/// - only=false 恒为 true（不限时段）；
/// - 支持跨午夜（如 22:00–06:00）；
/// - 时段解析失败时退化为 true，避免因配置异常而永久静默。
pub fn in_work_hours_at(only: bool, start: &str, end: &str, cur: u16) -> bool {
    if !only {
        return true;
    }
    match (parse_hm(start), parse_hm(end)) {
        (Some(s), Some(e)) => {
            if s <= e {
                cur >= s && cur <= e
            } else {
                cur >= s || cur <= e
            }
        }
        _ => true,
    }
}

impl AppState {
    /// 当前是否处于“工作时段”（基于本机当前时间）。
    pub fn in_work_hours(&self) -> bool {
        let now = chrono::Local::now();
        let cur = (now.hour() * 60 + now.minute()) as u16;
        in_work_hours_at(
            self.settings.work_hours_only,
            &self.settings.work_start,
            &self.settings.work_end,
            cur,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> AppState {
        AppState {
            user_state: UserState::Idle,
            daily: DailyData::default(),
            settings: Settings::default(),
            segment_start_ms: 0,
            rest_fired_for_segment: false,
            snooze_until_ms: 0,
            rest_segment_start_ms: 0,
            current_date: today_string(),
            water_deferred: false,
            last_save_ms: 0,
        }
    }

    #[test]
    fn on_activity_idle_to_working() {
        let mut s = make_state();
        s.on_activity(1000);
        assert_eq!(s.user_state, UserState::Working);
        assert_eq!(s.segment_start_ms, 1000);
    }

    #[test]
    fn tick_transitions_to_resting_after_inactivity() {
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1000;
        s.daily.last_activity_ms = 1000; // 休息阈值默认 10 分钟
        let old = s.tick(1000 + 11 * 60_000); // 超过 10 分钟无活动
        assert!(old.is_none());
        assert_eq!(s.user_state, UserState::Resting);
    }

    #[test]
    fn tick_stays_working_within_rest_threshold() {
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1000;
        s.daily.last_activity_ms = 1000;
        let old = s.tick(1000 + 5 * 60_000); // 仅 5 分钟
        assert!(old.is_none());
        assert_eq!(s.user_state, UserState::Working);
    }

    #[test]
    fn close_work_segment_records_duration() {
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1_000_000;
        s.close_work_segment(1_000_000 + 90_000); // 90 秒
        assert_eq!(s.daily.work_segments.len(), 1);
        assert_eq!(s.daily.work_segments[0].duration_sec, 90);
        assert_eq!(s.segment_start_ms, 0);
    }

    #[test]
    fn current_segment_ms_returns_zero_when_not_working() {
        let mut s = make_state();
        s.segment_start_ms = 5_000_000;
        // Idle 状态，即使有 segment_start 也不计时
        assert_eq!(s.current_segment_ms(5_000_000 + 10_000), 0);
    }

    #[test]
    fn current_segment_ms_counts_working() {
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 5_000_000;
        assert_eq!(s.current_segment_ms(5_000_000 + 30_000), 30_000);
    }

    #[test]
    fn daily_rollover_returns_old_and_resets() {
        let mut s = make_state();
        s.current_date = "2000-01-01".to_string();
        s.daily.date = "2000-01-01".to_string();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1000;
        let old = s.tick(now_ms());
        let old = old.expect("应返回归档的旧日数据");
        assert_eq!(old.date, "2000-01-01");
        assert_eq!(s.current_date, today_string());
        assert_eq!(s.user_state, UserState::Idle);
        assert_eq!(s.segment_start_ms, 0);
    }

    #[test]
    fn in_work_hours_at_normal_range() {
        assert!(in_work_hours_at(false, "09:00", "18:00", 720));
        assert!(in_work_hours_at(true, "09:00", "18:00", 720)); // 12:00
        assert!(in_work_hours_at(true, "09:00", "18:00", 540)); // 09:00 边界
        assert!(in_work_hours_at(true, "09:00", "18:00", 1080)); // 18:00 边界
        assert!(!in_work_hours_at(true, "09:00", "18:00", 480)); // 08:00
        assert!(!in_work_hours_at(true, "09:00", "18:00", 1200)); // 20:00
    }

    #[test]
    fn in_work_hours_at_cross_midnight() {
        assert!(in_work_hours_at(true, "22:00", "06:00", 1380)); // 23:00
        assert!(in_work_hours_at(true, "22:00", "06:00", 120)); // 02:00
        assert!(in_work_hours_at(true, "22:00", "06:00", 1320)); // 22:00 边界
        assert!(in_work_hours_at(true, "22:00", "06:00", 360)); // 06:00 边界
        assert!(!in_work_hours_at(true, "22:00", "06:00", 720)); // 12:00
    }

    #[test]
    fn in_work_hours_at_invalid_falls_back() {
        // 解析失败时退化为 true（不限时段），避免永久静默
        assert!(in_work_hours_at(true, "bad", "18:00", 720));
        assert!(in_work_hours_at(true, "09:00", "bad", 720));
    }
}
