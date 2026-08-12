use crate::state::{DailyData, Settings};
use std::fs;
use std::path::PathBuf;

/// 本地数据根目录：%LOCALAPPDATA%/rest-reminder
pub fn data_dir() -> PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("rest-reminder");
    let _ = fs::create_dir_all(&p);
    p
}

fn daily_dir() -> PathBuf {
    let mut p = data_dir();
    p.push("data");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn load_settings() -> Settings {
    let p = data_dir().join("settings.json");
    match fs::read_to_string(p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(s: &Settings) {
    let p = data_dir().join("settings.json");
    if let Ok(j) = serde_json::to_string_pretty(s) {
        if let Err(e) = fs::write(&p, j) {
            eprintln!(
                "[storage] 写入 settings.json 失败（设置可能未保存）: {:?}",
                e
            );
        }
    }
}

pub fn load_daily(date: &str) -> DailyData {
    let p = daily_dir().join(format!("{}.json", date));
    match fs::read_to_string(p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| DailyData {
            date: date.into(),
            ..Default::default()
        }),
        Err(_) => DailyData {
            date: date.into(),
            ..Default::default()
        },
    }
}

pub fn save_daily(d: &DailyData) {
    let p = daily_dir().join(format!("{}.json", d.date));
    if let Ok(j) = serde_json::to_string_pretty(d) {
        if let Err(e) = fs::write(&p, j) {
            eprintln!(
                "[storage] 写入 {} 失败（当日数据可能未保存）: {:?}",
                p.display(),
                e
            );
        }
    }
}

pub fn clear_daily(date: &str) {
    let p = daily_dir().join(format!("{}.json", date));
    let _ = fs::remove_file(p);
}
