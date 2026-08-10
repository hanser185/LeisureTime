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
        // 启动即视为全新片段：不沿用历史 last_activity_ms 作为起点，
        // 避免重启后误判“已连续工作很久”而立即弹提醒。
        let now = now_ms();
        // 初始化喝水提醒基准时间，避免首次工作就立即弹喝水提醒（Bug 2 修复）
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
