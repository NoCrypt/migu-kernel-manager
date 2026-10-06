use crate::util::{read_str, root, rooted};
use serde_json::{json, Value};
use std::io::Write;

const ALLOWED: &[&str] = &[
    "/sys",
    "/proc/sys",
    "/dev/cpuset",
    "/dev/stune",
    "/dev/cpuctl",
];

pub fn allowed_logical(path: &str) -> bool {
    if path.contains("..") || !path.starts_with('/') {
        return false;
    }
    ALLOWED
        .iter()
        .any(|r| path == *r || path.starts_with(&format!("{}/", r)))
}

pub fn write_value(path: &str, value: &str) -> Value {
    if !allowed_logical(path) {
        return json!({"ok": false, "requested": value, "actual": Value::Null,
                      "error": "path not in allowlist"});
    }
    let rp = rooted(path);

    if let Some(r) = root() {
        if let Ok(canon) = std::fs::canonicalize(&rp) {
            if let Ok(croot) = std::fs::canonicalize(&r) {
                if !canon.starts_with(&croot) {
                    return json!({"ok": false, "requested": value, "actual": Value::Null,
                                  "error": "path escapes root"});
                }
            }
        }
    }

    let res = std::fs::OpenOptions::new()
        .write(true)
        .open(&rp)
        .and_then(|mut f| f.write_all(value.as_bytes()));

    if let Err(e) = res {
        return json!({"ok": false, "requested": value, "actual": Value::Null,
                      "error": e.to_string()});
    }

    let actual = read_str(path).unwrap_or_default();
    let ok = actual == value
        || actual == format!("{} ", value)
        || actual == format!("{}\n", value);
    json!({"ok": ok, "requested": value, "actual": actual})
}
