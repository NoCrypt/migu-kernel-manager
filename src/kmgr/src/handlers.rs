use crate::registry::{self, Entry, Registry};
use crate::store;
use crate::util::{list_dir, read_str, rooted};
use crate::write::write_value;
use serde_json::{json, Value};
use std::thread::sleep;
use std::time::Duration;

// ------------------------------------------------------------------ helpers

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    match std::process::Command::new(cmd).args(args).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "{} {}: {}",
            cmd,
            args.join(" "),
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("{}: {}", cmd, e)),
    }
}

fn zram_dev() -> String {
    for cand in ["/dev/block/zram0", "/dev/zram0"] {
        if std::path::Path::new(&*rooted(cand)).exists() {
            return cand.to_string();
        }
    }
    "/dev/block/zram0".to_string()
}

fn bundle_members<'a>(reg: &'a Registry, scope: &str) -> Vec<&'a Entry> {
    let mut v = Vec::new();
    for g in &reg.groups {
        for s in &g.sections {
            if s.scope != scope {
                continue;
            }
            for e in &s.entries {
                if e.bundle.as_deref() == Some("zram") {
                    v.push(e);
                }
            }
        }
    }
    v
}

fn zram_algo_token(raw: &str) -> String {
    raw.split_whitespace()
        .find(|t| t.starts_with('['))
        .map(|t| t.trim_matches(|c| c == '[' || c == ']').to_string())
        .unwrap_or_else(|| raw.trim().to_string())
}

// swapoff, reset, comp_algorithm, disksize, mkswap, swapon  (boot-safe order)
fn zram_sequence(enabled: &str, size: &str, algo: &str) -> Result<(), String> {
    let dev = zram_dev();
    let base = "/sys/block/zram0";

    let _ = run("swapoff", &[&dev]);
    let _ = write_value(&format!("{}/reset", base), "1");

    if enabled == "1" {
        if !algo.is_empty() {
            write_value(&format!("{}/comp_algorithm", base), algo);
        }
        if size.parse::<i64>().unwrap_or(0) > 0 {
            let r = write_value(&format!("{}/disksize", base), size);
            if !r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                return Err(format!(
                    "disksize: {}",
                    r.get("error").and_then(|v| v.as_str()).unwrap_or("rejected")
                ));
            }
        }
        run("mkswap", &[&dev])?;
        run("swapon", &[&dev])?;
    }
    Ok(())
}

fn persist_bundle(reg: &Registry, scope: &str, enabled: &str, size: &str, algo: &str) {
    for m in bundle_members(reg, scope) {
        let v = match m.id.as_str() {
            "state" => enabled,
            "size" => size,
            "algo" => algo,
            _ => continue,
        };
        store::persist(&m.key, v);
    }
}

pub fn zram_set(reg: &Registry, target: &Entry, value: &str) -> Value {
    let base = "/sys/block/zram0";
    let cur_enabled = read_str(&format!("{}/initstate", base)).unwrap_or_else(|| "0".into());
    let cur_size = read_str(&format!("{}/disksize", base)).unwrap_or_else(|| "0".into());
    let cur_algo =
        zram_algo_token(&read_str(&format!("{}/comp_algorithm", base)).unwrap_or_default());

    let mut enabled = cur_enabled.clone();
    let mut size = cur_size.clone();
    let mut algo = cur_algo.clone();
    match target.id.as_str() {
        "state" => enabled = value.to_string(),
        "size" => size = value.to_string(),
        "algo" => algo = value.to_string(),
        _ => {}
    }

    if let Err(e) = zram_sequence(&enabled, &size, &algo) {
        let _ = zram_sequence(&cur_enabled, &cur_size, &cur_algo);
        return json!({"ok": false, "key": target.key, "requested": value, "error": e});
    }

    persist_bundle(reg, &target.scope, &enabled, &size, &algo);

    json!({
        "ok": true,
        "key": target.key,
        "requested": value,
        "actual": {
            "state": read_str(&format!("{}/initstate", base)),
            "size": read_str(&format!("{}/disksize", base)),
            "algo": zram_algo_token(&read_str(&format!("{}/comp_algorithm", base)).unwrap_or_default()),
        }
    })
}

pub fn zram_apply_saved(reg: &Registry) -> Value {
    let applied = store::load_applied();
    let base = "/sys/block/zram0";
    let mut enabled = None;
    let mut size = None;
    let mut algo = None;
    for g in &reg.groups {
        for s in &g.sections {
            for e in &s.entries {
                if e.bundle.as_deref() != Some("zram") {
                    continue;
                }
                if let Some(v) = applied.get(&e.key) {
                    match e.id.as_str() {
                        "state" => enabled = Some(v.clone()),
                        "size" => size = Some(v.clone()),
                        "algo" => algo = Some(v.clone()),
                        _ => {}
                    }
                }
            }
        }
    }
    let enabled = enabled.unwrap_or_else(|| read_str(&format!("{}/initstate", base)).unwrap_or_else(|| "0".into()));
    let size = size.unwrap_or_else(|| read_str(&format!("{}/disksize", base)).unwrap_or_else(|| "0".into()));
    let algo = algo.unwrap_or_else(|| {
        zram_algo_token(&read_str(&format!("{}/comp_algorithm", base)).unwrap_or_default())
    });
    let algo = zram_algo_token(&algo);
    match zram_sequence(&enabled, &size, &algo) {
        Ok(()) => json!({"key": "memory.zram", "ok": true}),
        Err(e) => json!({"key": "memory.zram", "ok": false, "error": e}),
    }
}

// ------------------------------------------------------------------ thermal
fn screen_off() -> bool {
    for name in list_dir("/sys/class/backlight") {
        let p = format!("/sys/class/backlight/{}/actual_brightness", name);
        if let Some(v) = read_str(&p) {
            if v.trim() == "0" {
                return true;
            }
            return false;
        }
    }
    false
}

fn hold_flag() -> std::path::PathBuf {
    store::hold_flag_path()
}

pub fn hold_thermal(interval_ms: u64) -> i32 {
    let interval = interval_ms.max(2000);
    let reg = registry::resolve();
    let e = match reg.find_by_id("thermal_profile").into_iter().next() {
        Some(e) => e,
        None => return 1,
    };
    let path = match &e.path {
        Some(p) => p.clone(),
        None => return 1,
    };
    let saved = store::load_applied()
        .get(&e.key)
        .cloned()
        .or_else(|| store::load_defaults().get(&e.key).and_then(|v| v.as_str().map(String::from)));
    let value = match saved {
        Some(v) => v,
        None => return 1,
    };

    loop {
        if !hold_flag().exists() {
            return 0;
        }
        if !screen_off() {
            let cur = read_str(&path);
            if cur.as_deref() != Some(value.as_str()) {
                write_value(&path, &value);
            }
        }
        sleep(Duration::from_millis(interval));
    }
}

// ------------------------------------------------------------------ custom

fn custom_file() -> std::path::PathBuf {
    store::state_dir().join("custom.json")
}

fn load_custom_paths() -> Vec<String> {
    std::fs::read_to_string(custom_file())
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| {
            v.get("tunables")
                .and_then(|t| t.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        })
        .unwrap_or_default()
}

fn save_custom_paths(paths: &[String]) {
    let _ = std::fs::create_dir_all(store::state_dir());
    let v = json!({ "tunables": paths });
    let _ = std::fs::write(custom_file(), serde_json::to_string_pretty(&v).unwrap_or_default());
}

pub fn custom_list() -> Value {
    let applied = store::load_applied();
    let items: Vec<Value> = load_custom_paths()
        .iter()
        .map(|p| {
            let value = read_str(p);
            let kind = match &value {
                Some(v) if v.parse::<i64>().is_ok() => "int",
                Some(_) => "string",
                None => "missing",
            };
            json!({"path": p, "value": value, "type": kind, "persisted": applied.contains_key(p)})
        })
        .collect();
    json!({"ok": true, "tunables": items})
}

pub fn custom_add(path: &str) -> Value {
    if !crate::write::allowed_logical(path) {
        return json!({"ok": false, "error": "path is not in an allowed root"});
    }
    if !std::path::Path::new(&*rooted(path)).exists() {
        return json!({"ok": false, "error": "path does not exist"});
    }
    let mut paths = load_custom_paths();
    if paths.iter().any(|p| p == path) {
        return json!({"ok": true, "already": true});
    }
    paths.push(path.to_string());
    save_custom_paths(&paths);
    json!({"ok": true})
}

pub fn custom_remove(path: &str) -> Value {
    let mut paths = load_custom_paths();
    paths.retain(|p| p != path);
    save_custom_paths(&paths);
    json!({"ok": true})
}
