use crate::util::rooted;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn state_dir() -> PathBuf {
    rooted("/data/adb/kmgr")
}

fn ensure_dir() {
    let _ = std::fs::create_dir_all(state_dir());
    let _ = std::fs::create_dir_all(state_dir().join("logs"));
    let _ = std::fs::create_dir_all(state_dir().join("profiles"));
    let _ = std::fs::create_dir_all(state_dir().join("scripts"));
    let _ = std::fs::create_dir_all(state_dir().join("backups"));
}

fn applied_path() -> PathBuf {
    state_dir().join("applied.conf")
}

fn defaults_path() -> PathBuf {
    state_dir().join("defaults.json")
}

fn settings_path() -> PathBuf {
    state_dir().join("settings.json")
}

pub fn load_settings() -> Map<String, Value> {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn save_settings(m: &Map<String, Value>) {
    ensure_dir();
    let _ = std::fs::write(
        settings_path(),
        serde_json::to_string_pretty(&Value::Object(m.clone())).unwrap_or_default(),
    );
}

pub fn set_setting(key: &str, value: &str) {
    let mut m = load_settings();
    m.insert(key.to_string(), Value::String(value.to_string()));
    save_settings(&m);
}

pub fn unset_setting(key: &str) {
    let mut m = load_settings();
    m.remove(key);
    save_settings(&m);
}

pub fn load_applied() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    if let Ok(s) = std::fs::read_to_string(applied_path()) {
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                m.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    m
}

pub fn save_applied(m: &BTreeMap<String, String>) {
    ensure_dir();
    let mut s = String::new();
    for (k, v) in m {
        s.push_str(&format!("{}={}\n", k, v));
    }
    let _ = std::fs::write(applied_path(), s);
}

pub fn persist(key: &str, value: &str) {
    let mut m = load_applied();
    m.insert(key.to_string(), value.to_string());
    save_applied(&m);
}

pub fn unpersist(key: &str) {
    let mut m = load_applied();
    m.remove(key);
    save_applied(&m);
}

pub fn load_defaults() -> Map<String, Value> {
    std::fs::read_to_string(defaults_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

pub fn capture_default(key: &str, value: &str) {
    let mut d = load_defaults();
    if d.contains_key(key) {
        return;
    }
    ensure_dir();
    d.insert(key.to_string(), Value::String(value.to_string()));
    let _ = std::fs::write(
        defaults_path(),
        serde_json::to_string_pretty(&Value::Object(d)).unwrap_or_default(),
    );
}

pub fn hold_flag_path() -> PathBuf {
    state_dir().join("hold_thermal")
}

pub fn hold_enabled() -> bool {
    hold_flag_path().exists()
}

pub fn set_hold(on: bool) {
    ensure_dir();
    let p = hold_flag_path();
    if on {
        let _ = std::fs::write(&p, "1");
    } else {
        let _ = std::fs::remove_file(&p);
    }
}

pub fn load_cap_est() -> Option<(i64, f64)> {
    let s = std::fs::read_to_string(state_dir().join("cap_est")).ok()?;
    let mut it = s.split_whitespace();
    let level = it.next()?.parse().ok()?;
    let value = it.next()?.parse().ok()?;
    Some((level, value))
}

pub fn save_cap_est(level: i64, value: f64) {
    ensure_dir();
    let _ = std::fs::write(state_dir().join("cap_est"), format!("{} {}", level, value));
}

pub fn boot_count() -> i64 {
    std::fs::read_to_string(state_dir().join("boot_count"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

pub fn reset_boot_count() {
    ensure_dir();
    let _ = std::fs::write(state_dir().join("boot_count"), "0");
}

pub fn set_boot_failed(failed: bool) {
    ensure_dir();
    let p = state_dir().join("boot_failed");
    if failed {
        let _ = std::fs::write(p, "1");
    } else {
        let _ = std::fs::remove_file(p);
    }
}
