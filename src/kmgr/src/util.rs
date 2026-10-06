use std::path::PathBuf;
use std::sync::OnceLock;

pub fn root() -> Option<PathBuf> {
    std::env::var_os("KMGR_ROOT").map(PathBuf::from)
}

// kmgr runs as root on device, where many /proc/sys and /sys nodes are mode
// 0444 but still writable by the superuser. Cache the uid once.
fn is_root() -> bool {
    static R: OnceLock<bool> = OnceLock::new();
    *R.get_or_init(|| {
        std::fs::read_to_string(rooted("/proc/self/status"))
            .ok()
            .map(|s| {
                s.lines().any(|l| {
                    l.starts_with("Uid:")
                        && l.split_whitespace().nth(1) == Some("0")
                })
            })
            .unwrap_or(false)
    })
}

pub fn rooted(path: &str) -> PathBuf {
    match root() {
        Some(r) => r.join(path.trim_start_matches('/')),
        None => PathBuf::from(path),
    }
}

pub fn read_str(path: &str) -> Option<String> {
    std::fs::read_to_string(rooted(path))
        .ok()
        .map(|s| s.trim().to_string())
}

pub fn path_exists(path: &str) -> bool {
    rooted(path).exists()
}

pub fn is_dir(path: &str) -> bool {
    rooted(path).is_dir()
}

pub fn is_writable(path: &str) -> bool {
    let p = rooted(path);
    if is_root() {
        return p.exists();
    }
    match std::fs::metadata(&p) {
        Ok(m) => !m.permissions().readonly(),
        Err(_) => false,
    }
}

pub fn list_dir(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(rooted(path)) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str() {
                out.push(n.to_string());
            }
        }
    }
    out.sort();
    out
}

pub fn basename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

pub fn join(base: &str, name: &str) -> String {
    if base == "/" {
        format!("/{}", name)
    } else {
        format!("{}/{}", base, name)
    }
}

pub fn trailing_number(name: &str) -> i64 {
    let digits: String = name
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse().unwrap_or(i64::MAX)
}

pub fn scaled(value: i64, scale: i64, unit: &str) -> String {
    let s = if scale <= 1 { value } else { value / scale };
    format!("{} {}", s, unit)
}
