use serde_json::{json, Value};
use std::process::Command;

fn run_wm(args: &[&str]) -> Result<String, String> {
    match Command::new("wm").args(args).output() {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).to_string()),
        Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn parse_wh(s: &str) -> Option<String> {
    let (a, b) = s.trim().split_once('x')?;
    let w: i64 = a.trim().parse().ok()?;
    let h: i64 = b.trim().parse().ok()?;
    Some(format!("{}x{}", w, h))
}

fn parse_colon(out: &str, key: &str) -> Option<String> {
    out.lines()
        .find_map(|l| l.trim().strip_prefix(key).map(|r| r.trim().to_string()))
}

fn parsed_size(out: &str) -> (Option<String>, Option<String>) {
    (
        parse_colon(out, "Physical size:").and_then(|s| parse_wh(&s)),
        parse_colon(out, "Override size:").and_then(|s| parse_wh(&s)),
    )
}

fn parse_num(out: &str, key: &str) -> Option<i64> {
    parse_colon(out, key).and_then(|s| s.trim().parse().ok())
}

pub fn status() -> Value {
    let size_out = run_wm(&["size"]).unwrap_or_default();
    let den_out = run_wm(&["density"]).unwrap_or_default();
    let (phys_size, over_size) = parsed_size(&size_out);
    let phys_den = parse_num(&den_out, "Physical density:");
    let over_den = parse_num(&den_out, "Override density:");
    json!({
        "ok": true,
        "size": {
            "current": over_size.clone().or(phys_size.clone()),
            "physical": phys_size,
            "override": over_size,
        },
        "density": {
            "current": over_den.or(phys_den),
            "physical": phys_den,
            "override": over_den,
        }
    })
}

pub fn set_size(v: &str) -> Value {
    if parse_wh(v).is_none() {
        return json!({"ok": false, "error": "expected WxH, e.g. 1080x2400"});
    }
    match run_wm(&["size", v]) {
        Ok(_) => status(),
        Err(e) => json!({"ok": false, "error": e}),
    }
}

pub fn set_density(v: &str) -> Value {
    if v.trim().parse::<i64>().is_err() {
        return json!({"ok": false, "error": "expected a number"});
    }
    match run_wm(&["density", v]) {
        Ok(_) => status(),
        Err(e) => json!({"ok": false, "error": e}),
    }
}

pub fn reset() -> Value {
    let a = run_wm(&["size", "reset"]);
    let b = run_wm(&["density", "reset"]);
    if a.is_err() && b.is_err() {
        return json!({"ok": false, "error": a.err().unwrap_or_default()});
    }
    status()
}

pub fn reset_size() -> Value {
    match run_wm(&["size", "reset"]) {
        Ok(_) => status(),
        Err(e) => json!({"ok": false, "error": e}),
    }
}

pub fn reset_density() -> Value {
    match run_wm(&["density", "reset"]) {
        Ok(_) => status(),
        Err(e) => json!({"ok": false, "error": e}),
    }
}
