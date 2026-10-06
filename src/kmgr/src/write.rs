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

// Linux O_NOFOLLOW: refuse to follow a symlink in the final path component.
#[cfg(unix)]
const O_NOFOLLOW: i32 = 0o400000;

// Resolving a path with realpath and then re-opening the original string leaves
// a TOCTOU window. We always open the canonical target, and O_NOFOLLOW makes the
// kernel reject it if the final component was swapped for a symlink in between.
#[cfg(unix)]
fn open_write_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
}

#[cfg(not(unix))]
fn open_write_nofollow(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new().write(true).open(path)
}

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

    // Resolve first, verify containment, then open the resolved target.
    let mut target = rp.clone();
    if let Ok(canon) = std::fs::canonicalize(&rp) {
        if let Some(r) = root() {
            if let Ok(croot) = std::fs::canonicalize(&r) {
                if !canon.starts_with(&croot) {
                    return json!({"ok": false, "requested": value, "actual": Value::Null,
                                  "error": "path escapes root"});
                }
            }
        }
        target = canon;
    }

    let res = open_write_nofollow(&target).and_then(|mut f| f.write_all(value.as_bytes()));

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
