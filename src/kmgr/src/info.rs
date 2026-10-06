use crate::registry;
use crate::util::{path_exists, read_str};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn getprop_all() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let out = std::process::Command::new("getprop").output();
    if let Ok(out) = out {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix('[') {
                if let Some((k, v)) = rest.split_once("]: [") {
                    if let Some(v) = v.strip_suffix(']') {
                        m.insert(k.to_string(), v.to_string());
                    }
                }
            }
        }
    }
    m
}

fn parse_gl(v: &str) -> Option<String> {
    let n: u32 = v.trim().parse().ok()?;
    Some(format!("OpenGL ES {}.{}", n >> 16, n & 0xffff))
}

// SurfaceFlinger prints: "GLES: <vendor>, <renderer>, <version-with-commas>".
fn gles_from_surfaceflinger() -> Option<(String, String, String)> {
    let out = std::process::Command::new("dumpsys")
        .arg("SurfaceFlinger")
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if let Some(rest) = line.trim().strip_prefix("GLES:") {
            let parts: Vec<&str> = rest.splitn(3, ',').collect();
            if parts.len() == 3 {
                return Some((
                    parts[0].trim().to_string(),
                    parts[1].trim().to_string(),
                    parts[2].trim().to_string(),
                ));
            }
        }
    }
    None
}

// "Adreno650v3" or "Adreno (TM) 650" -> "Adreno (TM) 650".
fn normalize_adreno(s: &str) -> Option<String> {
    if !s.to_lowercase().contains("adreno") {
        return None;
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).take(3).collect();
    if digits.is_empty() {
        None
    } else {
        Some(format!("Adreno (TM) {}", digits))
    }
}

pub fn run() -> i32 {
    let reg = registry::resolve();
    let src = reg.resolved_sources();
    let props = getprop_all();

    let kernel = read_str("/proc/version").unwrap_or_default();
    let gpu_model = src
        .get("gpu_model")
        .and_then(|p| read_str(p))
        .unwrap_or_default();
    let wireguard = src
        .get("wireguard_version")
        .and_then(|p| read_str(p))
        .unwrap_or_default();

    let gles = gles_from_surfaceflinger();

    let renderer = gles
        .as_ref()
        .map(|g| g.1.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| normalize_adreno(&gpu_model))
        .or_else(|| Some(gpu_model.clone()).filter(|s| !s.is_empty()))
        .unwrap_or_default();

    let vendor = gles
        .as_ref()
        .map(|g| g.0.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if path_exists("/sys/class/kgsl/kgsl-3d0") {
                "Qualcomm".to_string()
            } else {
                props.get("ro.hardware").cloned().unwrap_or_default()
            }
        });

    let gl_version = gles
        .as_ref()
        .map(|g| g.2.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            props
                .get("ro.opengles.version")
                .and_then(|v| parse_gl(v))
        })
        .unwrap_or_default();

    let out: Value = json!({
        "ok": true,
        "schema": crate::SCHEMA_VERSION,
        "kernel": kernel,
        "gpu": {
            "vendor": vendor,
            "renderer": renderer,
            "gl_version": gl_version,
        },
        "android": props.get("ro.build.version.release"),
        "sdk": props.get("ro.build.version.sdk"),
        "model": props.get("ro.product.model"),
        "device": props.get("ro.product.device"),
        "wireguard": wireguard,
        "selinux": props.get("ro.boot.selinux"),
        "slot": props.get("ro.boot.slot_suffix"),
    });
    println!("{}", serde_json::to_string(&out).unwrap_or_default());
    0
}
