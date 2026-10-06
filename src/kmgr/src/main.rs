mod display;
mod handlers;
mod info;
mod live;
mod monitor;
mod props;
mod registry;
mod store;
mod util;
mod write;

use registry::{Kind, Registry};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::process::exit;

// Bumped whenever the JSON contract changes shape. The UI compares this with the
// value baked into its own bundle to detect a mismatched binary after an update.
pub const SCHEMA_VERSION: u32 = 2;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        out(&json!({"ok": false, "error": "no command", "code": 2}));
        exit(2);
    }
    let cmd = args[0].as_str();
    let rest = &args[1..];
    let code = match cmd {
        "tree" => cmd_tree(rest),
        "get" => cmd_get(rest),
        "get-many" => cmd_get_many(rest),
        "set" => cmd_set(rest),
        "set-many" => cmd_set_many(rest),
        "set-path" => cmd_set_path(rest),
        "unpersist" => cmd_unpersist(rest),
        "persist" => cmd_persist(rest),
        "apply" => cmd_apply(rest),
        "reset" => cmd_reset(rest),
        "info" => cmd_info(),
        "version" | "--version" => cmd_version(),
        "discover" => cmd_discover(),
        "monitor" => cmd_monitor(rest),
        "live" => live::run(rest),
        "kill" => live::kill_process(rest),
        "dmesg" => live::dmesg_run(rest),
        "hold-thermal" => cmd_hold_thermal(rest),
        "thermal-hold" => cmd_thermal_hold(rest),
        "custom" => cmd_custom(rest),
        "wm" => cmd_wm(rest),
        "props" => cmd_props(rest),
        "settings" => cmd_settings(rest),
        "persist-path" => cmd_persist_path(rest),
        "unpersist-path" => cmd_unpersist_path(rest),
        "-h" | "--help" | "help" => {
            print_help();
            0
        }
        other => {
            out(&json!({"ok": false, "error": format!("unknown command '{}'", other), "code": 2}));
            2
        }
    };
    exit(code);
}


fn print_help() {
    println!(
        "kmgr <tree|get|get-many|set|set-many|set-path|persist|unpersist|persist-path|unpersist-path|custom|apply|reset|info|version|discover|monitor|live|kill|dmesg|wm|props|settings|hold-thermal|thermal-hold>"
    );
}

fn out(v: &Value) {
    println!("{}", serde_json::to_string(v).unwrap_or_else(|_| "{}".into()));
}

fn cmd_tree(args: &[String]) -> i32 {
    let reg = registry::resolve();
    capture_defaults(&reg);
    if args.iter().any(|a| a == "--values") {
        out(&json!({
            "ok": true,
            "schema": SCHEMA_VERSION,
            "values": reg.values_map(),
        }));
        return 0;
    }
    out(&reg.to_json());
    0
}

fn cmd_version() -> i32 {
    out(&json!({
        "ok": true,
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "schema": SCHEMA_VERSION,
    }));
    0
}

fn capture_defaults(reg: &Registry) {
    for g in &reg.groups {
        for s in &g.sections {
            for e in &s.entries {
                if let Some(v) = registry::effective_value(e) {
                    store::capture_default(&e.key, &v);
                }
            }
        }
    }
}

fn find_entry<'a>(reg: &'a Registry, key: &str) -> Option<&'a registry::Entry> {
    if let Some(e) = reg.find(key) {
        return Some(e);
    }
    let matches = reg.find_by_id(key);
    if matches.len() == 1 {
        return matches.first().copied();
    }
    None
}

fn cmd_get(args: &[String]) -> i32 {
    let key = match args.first() {
        Some(k) => k,
        None => {
            out(&json!({"ok": false, "error": "usage: kmgr get <id>", "code": 2}));
            return 2;
        }
    };
    let reg = registry::resolve();
    match find_entry(&reg, key) {
        Some(e) => {
            let v = registry::effective_value(e).unwrap_or_default();
            store::capture_default(&e.key, &v);
            out(&json!({
                "id": e.id, "key": e.key, "label": e.label, "type": e.kind.as_str(),
                "path": e.path, "value": v,
                "persisted": store::load_applied().contains_key(&e.key),
            }));
            0
        }
        None => {
            out(&json!({"ok": false, "error": format!("entry '{}' not found", key), "code": 1}));
            1
        }
    }
}

// Cheap refresh: only key -> value, no labels/choices/paths. Also accepts
// absolute paths for custom/submenu children.
fn cmd_get_many(args: &[String]) -> i32 {
    if args.is_empty() {
        out(&json!({"ok": false, "error": "usage: kmgr get-many <key>...", "code": 2}));
        return 2;
    }
    let reg = registry::resolve();
    let mut values = serde_json::Map::new();
    for key in args {
        let v = if key.starts_with('/') {
            util::read_str(key)
        } else {
            find_entry(&reg, key).and_then(registry::effective_value)
        };
        values.insert(
            key.clone(),
            v.map(Value::String).unwrap_or(Value::Null),
        );
    }
    out(&json!({"ok": true, "schema": SCHEMA_VERSION, "values": values}));
    0
}

fn cmd_set(args: &[String]) -> i32 {
    if args.len() < 2 {
        out(&json!({"ok": false, "error": "usage: kmgr set <id> <value> [--persist]", "code": 2}));
        return 2;
    }
    let key = &args[0];
    let value = &args[1];
    let persist = args.iter().any(|a| a == "--persist");

    let reg = registry::resolve();
    // First-seen defaults must be recorded before the first write, not on the
    // next UI refresh, or reset restores the value we just changed.
    capture_defaults(&reg);
    let e = match find_entry(&reg, key) {
        Some(e) => e,
        None => {
            out(&json!({"ok": false, "error": format!("entry '{}' not found", key), "code": 1}));
            return 1;
        }
    };

    if let Some(h) = &e.handler {
        if h == "zram" {
            let r = handlers::zram_set(&reg, e, value);
            let ok = r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            out(&r);
            return if ok { 0 } else { 1 };
        }
        out(&json!({"ok": false, "error": format!("entry is handled by native handler '{}'", h), "code": 1}));
        return 1;
    }
    if e.kind == Kind::Custom {
        out(&json!({"ok": false, "error": "entry has no writable node", "code": 1}));
        return 1;
    }
    let path = match &e.path {
        Some(p) => p.clone(),
        None => {
            out(&json!({"ok": false, "error": "entry has no path", "code": 1}));
            return 1;
        }
    };

    let mut result = write::write_value(&path, value);
    if persist {
        if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            store::persist(&e.key, value);
            result["persisted"] = json!(true);
        } else {
            result["persisted"] = json!(false);
        }
    }
    result["key"] = json!(e.key);
    let ok = result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    out(&result);
    if ok {
        0
    } else {
        1
    }
}

// Applies several key/value pairs as one unit. If any write fails (or is
// rejected by read-back), the already-written nodes are restored to their
// previous values, so bundles like KCAL RGB never end up half-applied.
fn cmd_set_many(args: &[String]) -> i32 {
    let persist = args.iter().any(|a| a == "--persist");
    let positional: Vec<&String> = args.iter().filter(|a| a.as_str() != "--persist").collect();
    if positional.is_empty() || positional.len() % 2 != 0 {
        out(&json!({"ok": false, "error": "usage: kmgr set-many <key> <value> ... [--persist]", "code": 2}));
        return 2;
    }
    let reg = registry::resolve();
    capture_defaults(&reg);

    let mut undo: Vec<(String, Option<String>)> = Vec::new();
    let mut applied: Vec<Value> = Vec::new();
    let mut failed: Option<Value> = None;

    for pair in positional.chunks(2) {
        let key = pair[0].as_str();
        let value = pair[1].as_str();
        let (path, persist_key, prior) = if key.starts_with('/') {
            (key.to_string(), key.to_string(), util::read_str(key))
        } else {
            match find_entry(&reg, key) {
                Some(e) if e.handler.is_none() && e.path.is_some() => {
                    let p = e.path.clone().unwrap();
                    (p.clone(), e.key.clone(), util::read_str(&p))
                }
                Some(_) => {
                    failed = Some(json!({"key": key, "ok": false, "error": "entry is not a plain writable node"}));
                    break;
                }
                None => {
                    failed = Some(json!({"key": key, "ok": false, "error": "not resolved on this kernel"}));
                    break;
                }
            }
        };

        let mut r = write::write_value(&path, value);
        let ok = r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            r["key"] = json!(key);
            failed = Some(r);
            break;
        }
        undo.push((path.clone(), prior));
        if persist {
            store::persist(&persist_key, value);
        }
        r["key"] = json!(key);
        applied.push(r);
    }

    if let Some(f) = failed {
        for (path, prior) in undo.iter().rev() {
            if let Some(v) = prior {
                write::write_value(path, v);
            }
        }
        out(&json!({"ok": false, "rolled_back": true, "failed": f, "applied": applied}));
        return 1;
    }
    out(&json!({"ok": true, "applied": applied}));
    0
}

fn cmd_set_path(args: &[String]) -> i32 {
    if args.len() < 2 {
        out(&json!({"ok": false, "error": "usage: kmgr set-path <path> <value>", "code": 2}));
        return 2;
    }
    if let Some(v) = util::read_str(&args[0]) {
        store::capture_default(&args[0], &v);
    }
    let mut r = write::write_value(&args[0], &args[1]);
    r["path"] = json!(args[0]);
    let ok = r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    out(&r);
    if ok {
        0
    } else {
        1
    }
}

fn cmd_unpersist(args: &[String]) -> i32 {
    let key = match args.first() {
        Some(k) => k,
        None => {
            out(&json!({"ok": false, "error": "usage: kmgr unpersist <id>"}));
            return 2;
        }
    };
    store::unpersist(key);
    out(&json!({"ok": true, "key": key}));
    0
}

fn cmd_persist(args: &[String]) -> i32 {
    let key = match args.first() {
        Some(k) => k,
        None => {
            out(&json!({"ok": false, "error": "usage: kmgr persist <id>"}));
            return 2;
        }
    };
    let reg = registry::resolve();
    let e = match find_entry(&reg, key) {
        Some(e) => e,
        None => {
            out(&json!({"ok": false, "error": format!("entry '{}' not found", key)}));
            return 1;
        }
    };
    if let Some(v) = registry::effective_value(e) {
        store::persist(&e.key, &v);
        out(&json!({"ok": true, "key": e.key, "value": v}));
        0
    } else {
        out(&json!({"ok": false, "error": "entry has no value to persist"}));
        1
    }
}

fn cmd_hold_thermal(args: &[String]) -> i32 {
    let mut interval: u64 = 10000;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--interval" && i + 1 < args.len() {
            interval = args[i + 1].parse().unwrap_or(10000);
            i += 2;
        } else {
            i += 1;
        }
    }
    handlers::hold_thermal(interval)
}

fn cmd_custom(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("list") | None => out(&handlers::custom_list()),
        Some("add") if args.len() >= 2 => out(&handlers::custom_add(&args[1])),
        Some("remove") if args.len() >= 2 => out(&handlers::custom_remove(&args[1])),
        _ => {
            out(&json!({"ok": false, "error": "usage: kmgr custom list|add|remove <path>"}));
            return 2;
        }
    }
    0
}

fn cmd_props(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("list") => {
            let q = args.get(1).map(String::as_str).unwrap_or("");
            out(&props::list(q));
        }
        Some("set") if args.len() >= 3 => {
            let key = &args[1];
            let value = args[2..].join(" ");
            out(&props::set(key, &value));
        }
        Some("unset") if args.len() >= 2 => out(&props::unset(&args[1])),
        Some("reset") => out(&props::reset()),
        _ => {
            out(&json!({"ok": false, "error": "usage: kmgr props [list [query]|set <key> <value>|unset <key>|reset]"}));
            return 2;
        }
    }
    0
}

fn cmd_settings(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("list") => {
            out(&json!({"ok": true, "settings": store::load_settings()}));
        }
        Some("set") if args.len() >= 3 => {
            let key = &args[1];
            if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                out(&json!({"ok": false, "error": "invalid setting key"}));
                return 2;
            }
            let value = args[2..].join(" ");
            store::set_setting(key, &value);
            out(&json!({"ok": true, "key": key, "value": value}));
        }
        Some("unset") if args.len() >= 2 => {
            store::unset_setting(&args[1]);
            out(&json!({"ok": true}));
        }
        _ => {
            out(&json!({"ok": false, "error": "usage: kmgr settings [list|set <key> <value>|unset <key>]"}));
            return 2;
        }
    }
    0
}

fn cmd_wm(args: &[String]) -> i32 {
    let sub = args.first().map(String::as_str);
    let arg = args.get(1).map(String::as_str);
    match (sub, arg) {
        (None, _) | (Some("status"), _) => out(&display::status()),
        (Some("size"), Some("reset")) => out(&display::reset_size()),
        (Some("size"), Some(v)) => out(&display::set_size(v)),
        (Some("density"), Some("reset")) => out(&display::reset_density()),
        (Some("density"), Some(v)) => out(&display::set_density(v)),
        (Some("reset"), _) => out(&display::reset()),
        _ => {
            out(&json!({"ok": false, "error": "usage: kmgr wm [status|reset|size <WxH>|size reset|density <N>|density reset]"}));
            return 2;
        }
    }
    0
}

fn cmd_persist_path(args: &[String]) -> i32 {
    let path = match args.first() {
        Some(p) => p,
        None => {
            out(&json!({"ok": false, "error": "usage: kmgr persist-path <path>"}));
            return 2;
        }
    };
    if !write::allowed_logical(path) {
        out(&json!({"ok": false, "error": "path is not in an allowed root"}));
        return 1;
    }
    let v = util::read_str(path).unwrap_or_default();
    store::persist(path, &v);
    out(&json!({"ok": true, "path": path, "value": v}));
    0
}

fn cmd_unpersist_path(args: &[String]) -> i32 {
    let path = match args.first() {
        Some(p) => p,
        None => {
            out(&json!({"ok": false, "error": "usage: kmgr unpersist-path <path>"}));
            return 2;
        }
    };
    store::unpersist(path);
    out(&json!({"ok": true, "path": path}));
    0
}

fn cmd_thermal_hold(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None => out(&json!({"ok": true, "on": store::hold_enabled()})),
        Some("on") => {
            store::set_hold(true);
            out(&json!({"ok": true, "on": true}));
        }
        Some("off") => {
            store::set_hold(false);
            out(&json!({"ok": true, "on": false}));
        }
        _ => {
            out(&json!({"ok": false, "error": "usage: kmgr thermal-hold [on|off]"}));
            return 2;
        }
    }
    0
}

fn cmd_apply(_args: &[String]) -> i32 {
    if store::boot_count() >= 2 {
        store::set_boot_failed(true);
        out(&json!({
            "ok": false, "skipped": true,
            "reason": "boot-loop guard: saved settings were skipped after a failed boot"
        }));
        return 3;
    }

    let reg = registry::resolve();
    // Record pre-apply values as the first-seen defaults before anything is
    // written; apply itself runs at boot, before the UI ever calls tree.
    capture_defaults(&reg);
    let applied = store::load_applied();
    let mut results: Vec<Value> = Vec::new();
    let mut handled: Vec<(String, String)> = Vec::new();
    let mut zram_seen = false;

    // Replay in registry order (groups -> sections -> entries), never file order,
    // so paired min/max nodes are written in the same order the UI writes them.
    // Keys that no longer resolve (or absolute paths) come last, in stable order.
    let mut ordered: Vec<(String, String)> = Vec::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for g in &reg.groups {
        for s in &g.sections {
            for e in &s.entries {
                if let Some(v) = applied.get(&e.key) {
                    ordered.push((e.key.clone(), v.clone()));
                    used.insert(e.key.clone());
                }
            }
        }
    }
    for (k, v) in &applied {
        if !used.contains(k) {
            ordered.push((k.clone(), v.clone()));
        }
    }

    for (key, value) in &ordered {
        if key.starts_with('/') {
            let mut r = write::write_value(key, value);
            r["key"] = json!(key);
            results.push(r);
            continue;
        }
        let e = match find_entry(&reg, key) {
            Some(e) => e,
            None => {
                results.push(json!({"key": key, "ok": false, "error": "not resolved on this kernel"}));
                continue;
            }
        };
        if let Some(h) = &e.handler {
            if h == "zram" {
                zram_seen = true;
            } else {
                handled.push((key.clone(), h.clone()));
            }
            continue;
        }
        let path = match &e.path {
            Some(p) => p.clone(),
            None => {
                results.push(json!({"key": key, "ok": false, "error": "no path"}));
                continue;
            }
        };
        let mut r = write::write_value(&path, value);
        r["key"] = json!(key);
        results.push(r);
    }

    if zram_seen {
        results.push(handlers::zram_apply_saved(&reg));
    }

    for (key, h) in handled {
        results.push(json!({"key": key, "ok": false, "error": format!("handler '{}' not implemented in this build", h)}));
    }

    let all_ok = results
        .iter()
        .all(|r| r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

    // The boot_count is NOT cleared here. service.sh clears it only after the
    // device has stayed up past a settle window, so a setting that crashes the
    // device shortly after a "successful" apply still trips the guard.
    out(&json!({"ok": all_ok, "applied": results}));
    0
}

fn cmd_reset(args: &[String]) -> i32 {
    let scope = args.first().map(String::as_str).unwrap_or("all");
    let reg = registry::resolve();
    let defaults = store::load_defaults();
    let applied = store::load_applied();

    let in_scope = |k: &str| -> bool {
        scope == "all" || k == scope || k.starts_with(&format!("{}.", scope))
    };

    let mut restored: Vec<Value> = Vec::new();
    for (key, value) in &defaults {
        if !in_scope(key) {
            continue;
        }
        if let Some(e) = find_entry(&reg, key) {
            if let Some(path) = &e.path {
                if e.handler.is_none() {
                    let mut r = write::write_value(path, value.as_str().unwrap_or(""));
                    r["key"] = json!(key);
                    restored.push(r);
                }
            }
        }
    }
    for key in applied.keys() {
        if in_scope(key) {
            store::unpersist(key);
        }
    }
    out(&json!({"ok": true, "scope": scope, "restored": restored}));
    0
}

fn cmd_info() -> i32 {
    info::run()
}

fn cmd_monitor(args: &[String]) -> i32 {
    let mut interval: u64 = 1000;
    let mut seconds: Option<u64> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--interval" && i + 1 < args.len() {
            interval = args[i + 1].parse().unwrap_or(1000);
            i += 2;
        } else if args[i] == "--seconds" && i + 1 < args.len() {
            seconds = args[i + 1].parse().ok();
            i += 2;
        } else {
            i += 1;
        }
    }
    monitor::run(interval, seconds)
}

fn cmd_discover() -> i32 {
    let log = store::state_dir().join("logs").join("discover.log");
    let content = std::fs::read_to_string(&log).unwrap_or_default();
    out(&json!({"ok": true, "log": log.to_string_lossy(), "content": content}));
    0
}
