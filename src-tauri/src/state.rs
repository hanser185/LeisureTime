use chrono::{NaiveTime, Timelike};
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

impl Settings {
    /// Normalize values that may come from an old or hand-edited settings file.
    pub fn sanitize(&mut self) {
        self.work_threshold_min = self.work_threshold_min.clamp(1, 1_440);
        self.rest_threshold_min = self.rest_threshold_min.clamp(1, 360);
        self.water_interval_min = self.water_interval_min.clamp(1, 1_440);

        if !matches!(
            self.reminder_mode.as_str(),
            "toast" | "popup" | "fullscreen"
        ) {
            self.reminder_mode = "toast".into();
        }
        if !matches!(self.theme.as_str(), "light" | "dark" | "system") {
            self.theme = "system".into();
        }
        if NaiveTime::parse_from_str(&self.work_start, "%H:%M").is_err() {
            self.work_start = "09:00".into();
        }
        if NaiveTime::parse_from_str(&self.work_end, "%H:%M").is_err() {
            self.work_end = "18:00".into();
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
    pub last_rest_prompt_ms: u64, // 上次休息提醒时刻，用于按工作阈值循环再提醒（修复“只弹一次”）
    pub pause_start_ms: u64, // 暂停起点；恢复时把时间锚点前移，避免暂停被算作无活动（修复“暂停误判休息”）
    pub last_tick_ms: u64,   // 上次调度 tick 时刻；用于检测睡眠/休眠导致的大幅时钟前跳
}

/// 调度 tick 名义间隔为 1s；相邻 tick 差值超过该阈值视为系统睡眠/休眠恢复，
/// 该段墙钟时间不计入工作/休息片段与各提醒基准（正常系统繁忙造成的 tick 抖动远小于此值）。
const SUSPEND_GAP_MS: u64 = 90_000;

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
            last_rest_prompt_ms: now,
            pause_start_ms: 0,
            last_tick_ms: now,
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

    /// 切换暂停态。
    /// 暂停时只记录起点；恢复时把所有时间锚点（活动/工作片段/休息片段/喝水提醒/休息提醒/稍后）
    /// 前移暂停时长，使暂停期间不被计为“无活动”而误判为休息（Bug2）。
    pub fn set_paused(&mut self, paused: bool, now: u64) {
        if paused {
            self.pause_start_ms = now;
        } else if self.pause_start_ms > 0 {
            let delta = now.saturating_sub(self.pause_start_ms);
            self.shift_anchors(delta);
            self.pause_start_ms = 0;
        }
        self.settings.paused = paused;
    }

    /// 把所有时间锚点整体前移 delta：暂停恢复与睡眠/休眠间隙补偿共用，
    /// 使这段墙钟时间既不计入工作/休息时长，也不算无活动、不推进提醒基准。
    fn shift_anchors(&mut self, delta: u64) {
        self.daily.last_activity_ms = self.daily.last_activity_ms.saturating_add(delta);
        self.daily.last_water_prompt_ms = self.daily.last_water_prompt_ms.saturating_add(delta);
        self.last_rest_prompt_ms = self.last_rest_prompt_ms.saturating_add(delta);
        if self.segment_start_ms > 0 {
            self.segment_start_ms = self.segment_start_ms.saturating_add(delta);
        }
        if self.rest_segment_start_ms > 0 {
            self.rest_segment_start_ms = self.rest_segment_start_ms.saturating_add(delta);
        }
        self.snooze_until_ms = self.snooze_until_ms.saturating_add(delta);
    }

    /// 每秒 tick：推进状态机；跨日返回需保存的旧日数据
    pub fn tick(&mut self, now: u64) -> Option<DailyData> {
        let prev_tick = self.last_tick_ms;
        self.last_tick_ms = now;

        if self.settings.paused {
            // 暂停期间的时钟跳变由 set_paused 恢复时统一补偿
            return None;
        }

        // 睡眠/休眠恢复检测：相邻 tick 间隔异常大，说明这段墙钟时间系统并未真正运行，
        // 前移全部锚点剔除该间隙，避免把睡眠时长算进工作/休息片段（或误判为休息）。
        // prev_tick == 0 表示尚无基准（测试构造或首 tick），不做补偿。
        if prev_tick > 0 {
            let gap = now.saturating_sub(prev_tick);
            if gap >= SUSPEND_GAP_MS {
                self.shift_anchors(gap);
            }
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
            self.daily.last_activity_ms = now;
            self.daily.last_water_prompt_ms = now;
            self.current_date = today;
            self.user_state = UserState::Idle;
            self.segment_start_ms = 0;
            self.rest_segment_start_ms = 0;
            self.rest_fired_for_segment = false;
            self.snooze_until_ms = 0;
            self.water_deferred = false;
            self.last_save_ms = now;
            self.last_rest_prompt_ms = now;
            self.pause_start_ms = 0;
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
                    // 注：休息次数仅在“弹出休息提醒”时由 scheduler 累加，
                    // 此处被动转入休息不再计数，避免与提醒重复累加（修复“休息次数恒为 0”）
                }
            }
            UserState::Resting => {
                if since < rest_th {
                    self.close_rest_segment(now);
                    self.user_state = UserState::Working;
                    self.segment_start_ms = now;
                    self.rest_fired_for_segment = false;
                } else {
                    // 超过休息阈值仍未主动恢复，视为完成休息片段，转回工作状态
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

    /// 记录一次喝水：追加时间并刷新下一轮喝水提醒起点（对应 record_water 命令核心）
    pub fn record_water(&mut self, hm: String) {
        self.daily.water_intakes.push(hm);
        self.daily.last_water_prompt_ms = now_ms();
    }

    /// 推迟喝水提醒一整个间隔（对应 defer_water 命令核心）
    pub fn defer_water(&mut self) {
        self.daily.last_water_prompt_ms = now_ms();
    }

    /// 休息弹窗关闭时复位：解除“喝水提醒让位”标记（Bug1 复位核心，main.rs 窗口事件调用）
    pub fn reset_water_defer(&mut self) {
        self.water_deferred = false;
    }

    /// 按工作阈值循环再提醒：距上次休息提醒已过去一个完整工作阈值，即解除本片段的提醒抑制，
    /// 使连续长时间工作能多次收到提醒（修复“休息提醒只弹一次”）。
    pub fn rearm_rest_if_due(&mut self, now: u64, work_th: u64) {
        if self.rest_fired_for_segment && now.saturating_sub(self.last_rest_prompt_ms) >= work_th {
            self.rest_fired_for_segment = false;
        }
    }

    /// 清空今日数据并复位在途状态；否则旧工作片段会被下次归档“复活”进空日（Bug3）。
    pub fn reset_daily_and_tracking(&mut self, now: u64) {
        self.daily = DailyData {
            date: self.current_date.clone(),
            ..Default::default()
        };
        self.daily.last_activity_ms = now;
        self.daily.last_water_prompt_ms = now;
        self.user_state = UserState::Idle;
        self.segment_start_ms = 0;
        self.rest_segment_start_ms = 0;
        self.rest_fired_for_segment = false;
        self.snooze_until_ms = 0;
        self.water_deferred = false;
        self.last_rest_prompt_ms = now;
        self.pause_start_ms = 0;
        self.last_save_ms = now;
    }
}

/// 供 Tauri 托管的包装
pub struct Store(pub Mutex<AppState>);

impl Store {
    pub fn new(settings: Settings, daily: DailyData, date: String) -> Self {
        Store(Mutex::new(AppState::new(settings, daily, date)))
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, AppState> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
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
            last_rest_prompt_ms: 0,
            pause_start_ms: 0,
            last_tick_ms: 0,
        }
    }

    #[test]
    fn settings_sanitize_clamps_ranges_and_resets_enum_values() {
        let mut s = Settings {
            work_threshold_min: 0,
            rest_threshold_min: 1_000,
            water_interval_min: 0,
            reminder_mode: "invalid".into(),
            theme: "invalid".into(),
            ..Settings::default()
        };

        s.sanitize();

        assert_eq!(s.work_threshold_min, 1);
        assert_eq!(s.rest_threshold_min, 360);
        assert_eq!(s.water_interval_min, 1);
        assert_eq!(s.reminder_mode, "toast");
        assert_eq!(s.theme, "system");
    }

    #[test]
    fn settings_sanitize_replaces_invalid_work_hour_strings() {
        let mut s = Settings {
            work_start: "bad".into(),
            work_end: "25:99".into(),
            ..Settings::default()
        };

        s.sanitize();

        assert_eq!(s.work_start, "09:00");
        assert_eq!(s.work_end, "18:00");
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
        assert_eq!(s.daily.last_activity_ms, s.last_save_ms);
        assert_eq!(s.daily.last_water_prompt_ms, s.last_save_ms);
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

    // ===== 回归测试：本次修复的状态逻辑 =====

    #[test]
    fn new_resets_activity_and_water_baseline() {
        // Bug3：启动不沿用历史 last_activity_ms / last_water_prompt_ms，
        // 否则可能误判“已连续工作很久”立即弹休息，或首次工作就弹喝水
        let s = AppState::new(Settings::default(), DailyData::default(), today_string());
        assert!(s.daily.last_activity_ms > 0, "启动应重置活动起点");
        assert!(s.daily.last_water_prompt_ms > 0, "启动应重置喝水提醒基准");
        assert!(!s.water_deferred);
    }

    #[test]
    fn record_water_resets_prompt_and_pushes() {
        // Bug2：记录喝水应追加时间点，并刷新下一轮提醒起点（不从“弹窗时刻”起算）
        let mut s = make_state();
        s.daily.last_water_prompt_ms = 1000; // 旧基准
        s.record_water("12:34".into());
        assert_eq!(
            s.daily.water_intakes.last().map(|x| x.as_str()),
            Some("12:34")
        );
        assert!(s.daily.last_water_prompt_ms > 1000, "喝水后提醒起点应刷新");
    }

    #[test]
    fn defer_water_resets_prompt() {
        let mut s = make_state();
        s.daily.last_water_prompt_ms = 0;
        s.defer_water();
        assert!(s.daily.last_water_prompt_ms > 0);
    }

    #[test]
    fn reset_water_defer_clears_flag() {
        // Bug1：休息弹窗关闭后 water_deferred 必须复位，否则当天喝水提醒永久失效
        let mut s = make_state();
        s.water_deferred = true;
        s.reset_water_defer();
        assert!(!s.water_deferred);
    }

    // ===== 本次修复的回归测试 =====

    fn make_working_state() -> AppState {
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1000;
        s.daily.last_activity_ms = 1000;
        s
    }

    #[test]
    fn set_paused_shifts_anchors_on_resume() {
        // Bug2：暂停期间不能更新活动锚点；恢复时把包括暂停时长的所有锚点前移，
        // 否则恢复瞬间 since 含暂停时长，跨过休息阈值会误判为“休息”并污染统计。
        let mut s = make_working_state();
        s.set_paused(true, 1000);
        // 暂停 10 分钟
        s.set_paused(false, 1000 + 10 * 60_000);
        // 活动锚点应前移 10 分钟
        assert_eq!(s.daily.last_activity_ms, 1000 + 10 * 60_000);
        assert_eq!(s.segment_start_ms, 1000 + 10 * 60_000);
        // 恢复后仅 1s 就 tick，不应因暂停时长误判为休息
        let old = s.tick(1000 + 10 * 60_000 + 1_000);
        assert!(old.is_none());
        assert_eq!(s.user_state, UserState::Working);
    }

    #[test]
    fn reset_daily_and_tracking_clears_inflight() {
        // Bug3：清空今日必须一并复位在途状态，否则旧工作片段会被下次归档“复活”进空日
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 5000;
        s.rest_segment_start_ms = 4000;
        s.rest_fired_for_segment = true;
        s.water_deferred = true;
        s.snooze_until_ms = 999;
        let now = 1_000_000;
        s.reset_daily_and_tracking(now);
        assert_eq!(s.user_state, UserState::Idle);
        assert_eq!(s.segment_start_ms, 0);
        assert_eq!(s.rest_segment_start_ms, 0);
        assert!(!s.rest_fired_for_segment);
        assert!(!s.water_deferred);
        assert_eq!(s.snooze_until_ms, 0);
        assert_eq!(s.daily.rest_count, 0);
        assert_eq!(s.daily.work_segments.len(), 0);
        assert_eq!(s.daily.last_activity_ms, now);
    }

    #[test]
    fn rearm_rest_if_due_releases_after_threshold() {
        // Bug1（修复“只弹一次”）：距上次提醒达一个工作阈值后解除抑制，以便连续工作可再次提醒
        let mut s = make_state();
        s.rest_fired_for_segment = true;
        s.last_rest_prompt_ms = 1_000_000;
        let work_th = 60 * 60_000;
        s.rearm_rest_if_due(1_000_000 + work_th - 1, work_th); // 差 1ms
        assert!(s.rest_fired_for_segment, "未达阈值应保持抑制");
        s.rearm_rest_if_due(1_000_000 + work_th, work_th); // 恰好阈值
        assert!(!s.rest_fired_for_segment, "达阈值应解除抑制以便再次提醒");
    }

    #[test]
    fn tick_passive_rest_does_not_increment_rest_count() {
        // Bug4：被动转入休息（idle 检测）不应计数，休息次数仅由“弹出提醒”累加，避免重复
        let mut s = make_working_state();
        s.tick(1000 + 11 * 60_000); // 超过 10 分钟无活动，转入 Resting
        assert_eq!(s.user_state, UserState::Resting);
        assert_eq!(s.daily.rest_count, 0, "被动转入休息不应计数");
    }

    // ===== 睡眠/休眠间隙补偿回归测试 =====

    #[test]
    fn suspend_gap_does_not_inflate_work_segment() {
        // 睡前已连续工作 30 分钟，合盖睡眠 8 小时后唤醒：
        // 唤醒后第一次调度 tick 应剔除睡眠间隙，片段时长不膨胀
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1000;
        s.last_tick_ms = 1000 + 30 * 60_000; // 睡前最后一次 tick：已连续工作 30 分钟
        s.daily.last_activity_ms = s.last_tick_ms; // 睡前一刻仍在活动
        let wake = s.last_tick_ms + 8 * 3_600_000;
        s.tick(wake); // 唤醒后的第一次调度 tick 完成间隙补偿
        assert_eq!(s.user_state, UserState::Working, "睡眠间隙不应计入无活动");
        assert_eq!(
            s.segment_start_ms,
            wake - 30 * 60_000,
            "工作锚点应前移剔除睡眠"
        );
        assert_eq!(
            s.current_segment_ms(wake + 60_000),
            31 * 60_000,
            "片段时长不应包含睡眠"
        );
    }

    #[test]
    fn suspend_gap_does_not_trigger_false_rest() {
        // 睡前已无活动 5 分钟，睡眠 2 小时后唤醒：等效无活动仍约 5 分钟，
        // 不应因墙钟跳变直接判为休息（10 分钟阈值）
        let mut s = make_state();
        s.user_state = UserState::Working;
        s.segment_start_ms = 1000;
        s.daily.last_activity_ms = 4000; // 睡前已无活动 5 分钟
        s.last_tick_ms = 9000; // 上一次 tick 在活动开始 9 秒后
        let wake = s.last_tick_ms + 2 * 3_600_000;
        s.tick(wake);
        assert_eq!(s.user_state, UserState::Working, "睡眠间隙不应计入无活动");
        assert_eq!(s.daily.last_activity_ms, wake - 5000);
    }

    #[test]
    fn suspend_gap_below_threshold_is_ignored() {
        // 正常系统繁忙导致的 tick 抖动（<90s）不触发锚点前移
        let mut s = make_working_state();
        s.last_tick_ms = 1000 + 60_000; // 与活动基准差 1 分钟
        let now = s.last_tick_ms + 80_000; // 间隙 80 秒，低于阈值
        s.tick(now);
        assert_eq!(s.daily.last_activity_ms, 1000, "小抖动不应移动锚点");
        assert_eq!(s.user_state, UserState::Working);
    }
}
