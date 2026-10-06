use crate::registry;
use crate::store;
use crate::util::{join, list_dir, path_exists, rooted, trailing_number};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::thread::sleep;
use std::time::Duration;

const CLOCK_MONOTONIC: i32 = 1;
const CLOCK_BOOTTIME: i32 = 7;

#[cfg(unix)]
#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(unix)]
extern "C" {
    fn clock_gettime(clk_id: i32, tp: *mut Timespec) -> i32;
}

#[cfg(unix)]
fn clock_secs(clk_id: i32) -> Option<f64> {
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { clock_gettime(clk_id, &mut ts) } == 0 {
        Some(ts.tv_sec as f64 + ts.tv_nsec as f64 / 1_000_000_000.0)
    } else {
        None
    }
}

// Host builds (the off-device smoke test) have no clock_gettime; there deep
// sleep is unavailable and falls back to suspend statistics.
#[cfg(not(unix))]
fn clock_secs(_clk_id: i32) -> Option<f64> {
    None
}

// Deep sleep (total suspend time) = CLOCK_BOOTTIME - CLOCK_MONOTONIC, the same
// pair Android exposes as SystemClock.elapsedRealtime() - uptimeMillis().
pub(crate) fn deep_sleep() -> Option<(f64, f64)> {
    let boot = clock_secs(CLOCK_BOOTTIME)?;
    let awake = clock_secs(CLOCK_MONOTONIC)?;
    if boot <= 0.0 {
        return None;
    }
    let d = (boot - awake).max(0.0);
    Some((d * 1000.0, (d / boot * 1000.0).round() / 10.0))
}

pub(crate) struct Handle {
    file: File,
    buf: Vec<u8>,
}

impl Handle {
    pub(crate) fn open(path: &str) -> Option<Handle> {
        File::open(rooted(path)).ok().map(|file| Handle {
            file,
            buf: Vec::with_capacity(512),
        })
    }
    pub(crate) fn read(&mut self) -> Option<String> {
        self.file.seek(SeekFrom::Start(0)).ok()?;
        self.buf.clear();
        self.file.read_to_end(&mut self.buf).ok()?;
        Some(String::from_utf8_lossy(&self.buf).trim().to_string())
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Times {
    pub(crate) busy: u64,
    pub(crate) total: u64,
}

pub(crate) fn parse_stat(s: &str) -> (Times, BTreeMap<usize, Times>) {
    let mut overall = Times::default();
    let mut cores = BTreeMap::new();
    for line in s.lines() {
        if !line.starts_with("cpu") {
            continue;
        }
        let mut it = line.split_whitespace();
        let tag = it.next().unwrap_or("");
        let vals: Vec<u64> = it.filter_map(|x| x.parse().ok()).collect();
        if vals.len() < 4 {
            continue;
        }
        let total: u64 = vals.iter().sum();
        let idle = vals[3] + vals.get(4).copied().unwrap_or(0);
        let t = Times {
            busy: total.saturating_sub(idle),
            total,
        };
        if tag == "cpu" {
            overall = t;
        } else if let Ok(n) = tag[3..].parse::<usize>() {
            cores.insert(n, t);
        }
    }
    (overall, cores)
}

pub(crate) fn busy_pct(prev: Times, now: Times) -> Option<f64> {
    let dt = now.total.saturating_sub(prev.total);
    if dt == 0 {
        return None;
    }
    let db = now.busy.saturating_sub(prev.busy);
    Some((db as f64) * 100.0 / (dt as f64))
}

pub(crate) fn first_num(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit() && *c != '-' && *c != '.')
        .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '.')
        .collect();
    cleaned.parse().ok()
}

pub(crate) fn meminfo_field(s: &str, key: &str) -> Option<f64> {
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{}:", key)) {
            return first_num(rest);
        }
    }
    None
}

struct Core {
    cpu: usize,
    cur: Handle,
    min: Handle,
    max: Handle,
}

struct Cluster {
    title: String,
    cur: Option<Handle>,
    max: Handle,
    governor: Handle,
}

pub fn run(interval_ms: u64, seconds: Option<u64>) -> i32 {
    let interval = interval_ms.max(200);
    let max_secs = seconds.unwrap_or(0);
    let started = std::time::Instant::now();
    let reg = registry::resolve();
    let src = reg.resolved_sources();

    let mut handles: BTreeMap<String, Handle> = BTreeMap::new();
    for (k, p) in &src {
        if let Some(h) = Handle::open(p) {
            handles.insert(k.clone(), h);
        }
    }

    let mut cores = discover_cores();
    let mut clusters = discover_clusters();

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    let mut prev_overall = handles
        .get_mut("cpu_stat")
        .and_then(|h| h.read())
        .map(|s| parse_stat(&s).0)
        .unwrap_or_default();
    let mut prev_cores: BTreeMap<usize, Times> = handles
        .get_mut("cpu_stat")
        .and_then(|h| h.read())
        .map(|s| parse_stat(&s).1)
        .unwrap_or_default();
    let (mut cap_smooth, mut cap_level) = match store::load_cap_est() {
        Some((lvl, val)) => (Some(val), Some(lvl)),
        None => (None, None),
    };

    loop {
        if max_secs > 0 && started.elapsed().as_secs() >= max_secs {
            return 0;
        }
        sleep(Duration::from_millis(interval));

        let (overall, core_times) = match handles.get_mut("cpu_stat").and_then(|h| h.read()) {
            Some(s) => parse_stat(&s),
            None => (Times::default(), BTreeMap::new()),
        };

        let mut cores_json: Vec<Value> = Vec::new();
        for c in cores.iter_mut() {
            let cur = c.cur.read();
            let min = c.min.read();
            let max = c.max.read();
            let load = match core_times.get(&c.cpu).zip(prev_cores.get(&c.cpu)) {
                Some((now, p)) => busy_pct(*p, *now),
                None => None,
            };
            cores_json.push(json!({
                "cpu": c.cpu,
                "cur_khz": cur.and_then(|v| v.parse::<i64>().ok()),
                "min_khz": min.and_then(|v| v.parse::<i64>().ok()),
                "max_khz": max.and_then(|v| v.parse::<i64>().ok()),
                "load": load.map(|v| (v * 10.0).round() / 10.0),
            }));
        }

        let clusters_json: Vec<Value> = clusters
            .iter_mut()
            .map(|c| {
                json!({
                    "title": c.title,
                    "governor": c.governor.read(),
                    "cur_khz": c.cur.as_mut().and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
                    "max_khz": c.max.read().and_then(|v| v.parse::<i64>().ok()),
                })
            })
            .collect();

        let load_pct = busy_pct(prev_overall, overall).map(|v| (v * 10.0).round() / 10.0);

        let mem = handles.get_mut("meminfo").and_then(|h| h.read()).map(|s| {
            json!({
                "total": meminfo_field(&s, "MemTotal").map(|v| v / 1024.0),
                "free": meminfo_field(&s, "MemFree").map(|v| v / 1024.0),
                "available": meminfo_field(&s, "MemAvailable").map(|v| v / 1024.0),
                "cached": meminfo_field(&s, "Cached").map(|v| v / 1024.0),
            })
        });

        let zram = {
            let disksize = handles
                .get_mut("zram_disksize")
                .and_then(|h| h.read())
                .and_then(|v| v.trim().parse::<f64>().ok());
            let mm = handles.get_mut("zram_mm_stat").and_then(|h| h.read());
            match (disksize, mm) {
                (Some(total_bytes), Some(s)) => {
                    let v: Vec<f64> =
                        s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    let total = total_bytes / 1048576.0;
                    let used = v.first().copied().unwrap_or(0.0) / 1048576.0;
                    let compr = v.get(1).copied().unwrap_or(0.0) / 1048576.0;
                    let pct = if total > 0.0 {
                        (used / total * 1000.0).round() / 10.0
                    } else {
                        0.0
                    };
                    Some(json!({"total": total, "used": used, "compr": compr, "pct": pct}))
                }
                _ => None,
            }
        };

        let batt_level = handles
            .get_mut("battery_capacity")
            .and_then(|h| h.read())
            .and_then(|v| v.parse::<i64>().ok());
        let batt_counter = handles
            .get_mut("battery_charge_counter")
            .and_then(|h| h.read())
            .and_then(|v| v.parse::<i64>().ok());
        // Full-capacity estimate from the coulomb counter. It is only re-sampled
        // when the integer SoC actually changes: between two percent steps the
        // counter keeps rising (esp. while charging) while the level does not, so
        // a per-tick ratio would creep upwards. Sampling at level transitions
        // keeps it steady, and the EWMA damps the odd noisy step.
        let cap_mah = match (batt_counter, batt_level) {
            (Some(cc), Some(level)) if level > 0 && cc > 0 => {
                if cap_level != Some(level) {
                    let raw = (cc as f64 / 1000.0) * 100.0 / level as f64;
                    let s = match cap_smooth {
                        Some(prev) => prev + (raw - prev) * 0.3,
                        None => raw,
                    };
                    cap_smooth = Some(s);
                    cap_level = Some(level);
                    store::save_cap_est(level, s);
                }
                cap_smooth.map(|s| s.round())
            }
            _ => cap_smooth.map(|s| s.round()),
        };

        let battery = json!({
            "capacity": batt_level,
            "capacity_mah": cap_mah,
            "status": handles.get_mut("battery_status").and_then(|h| h.read()),
            "temp": handles.get_mut("battery_temp").and_then(|h| h.read()).and_then(|v| first_num(&v)).map(|v| v / 10.0),
            "current": handles.get_mut("battery_current").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            "voltage": handles.get_mut("battery_voltage").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            "charge_counter": batt_counter,
            "charge_full": handles.get_mut("battery_charge_full").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            "charge_full_design": handles.get_mut("battery_charge_full_design").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            "cycles": handles.get_mut("battery_cycles").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
        });

        let gpu = json!({
            "busy": handles.get_mut("gpu_busy").and_then(|h| h.read()).and_then(|v| first_num(&v)),
            "cur_hz": handles.get_mut("gpu_cur_freq").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            "max_hz": handles.get_mut("gpu_max_freq").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
        });

        let loadavg_line = handles.get_mut("loadavg").and_then(|h| h.read());
        let loadavg = loadavg_line.as_ref().map(|s| {
            s.split_whitespace()
                .take(3)
                .filter_map(|x| x.parse::<f64>().ok())
                .collect::<Vec<f64>>()
        });

        let uptime = handles
            .get_mut("uptime")
            .and_then(|h| h.read())
            .and_then(|v| first_num(&v));

        // Prefer the kernel's own suspend accounting when it exists; otherwise
        // derive it from CLOCK_BOOTTIME - CLOCK_MONOTONIC.
        let suspend_ms = handles
            .get_mut("suspend_time")
            .and_then(|h| h.read())
            .and_then(|v| first_num(&v));
        let (deep_sleep_ms, deep_sleep_pct) = match suspend_ms {
            Some(ms) => {
                let pct = uptime.and_then(|up| {
                    let total = up * 1000.0 + ms;
                    if total > 0.0 {
                        Some((ms / total * 1000.0).round() / 10.0)
                    } else {
                        None
                    }
                });
                (Some(ms), pct)
            }
            None => match deep_sleep() {
                Some((ms, pct)) => (Some(ms), Some(pct)),
                None => (None, None),
            },
        };

        let sample = json!({
            "t": now_ms(),
            "load": load_pct,
            "cores": cores_json,
            "clusters": clusters_json,
            "cpu_temp": handles.get_mut("cpu_temp").and_then(|h| h.read()).and_then(|v| first_num(&v)).map(|v| v / 1000.0),
            "gpu": gpu,
            "mem": mem,
            "zram": zram,
            "battery": battery,
            "loadavg": loadavg,
            "loadavg_full": loadavg_line,
            "entropy": {
                "avail": handles.get_mut("entropy_avail").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
                "poolsize": handles.get_mut("entropy_poolsize").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            },
            "uptime": uptime,
            "suspend_count": handles.get_mut("suspend_count").and_then(|h| h.read()).and_then(|v| v.parse::<i64>().ok()),
            "deep_sleep": deep_sleep_pct,
            "deep_sleep_ms": deep_sleep_ms,
        });

        if writeln!(lock, "{}", serde_json::to_string(&sample).unwrap_or_default()).is_err() {
            return 0;
        }
        if lock.flush().is_err() {
            return 0;
        }

        prev_overall = overall;
        prev_cores = core_times;
    }
}

fn discover_cores() -> Vec<Core> {
    let base = "/sys/devices/system/cpu";
    let mut out = Vec::new();
    let mut names: Vec<String> = list_dir(base)
        .into_iter()
        .filter(|n| n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit()))
        .collect();
    names.sort_by_key(|n| trailing_number(n));
    for n in names {
        let cpu: usize = match n[3..].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let dir = join(base, &n);
        let cur_p = join(&dir, "cpufreq/scaling_cur_freq");
        let min_p = join(&dir, "cpufreq/cpuinfo_min_freq");
        let max_p = join(&dir, "cpufreq/cpuinfo_max_freq");
        if !path_exists(&cur_p) {
            continue;
        }
        if let (Some(cur), Some(min), Some(max)) = (
            Handle::open(&cur_p),
            Handle::open(&min_p),
            Handle::open(&max_p),
        ) {
            out.push(Core { cpu, cur, min, max });
        }
    }
    out
}

fn discover_clusters() -> Vec<Cluster> {
    let base = "/sys/devices/system/cpu/cpufreq";
    let titles = ["Little cluster", "Big cluster", "Prime cluster"];
    let mut pols: Vec<String> = list_dir(base)
        .into_iter()
        .filter(|n| n.starts_with("policy"))
        .collect();
    pols.sort_by_key(|n| trailing_number(n));
    let mut out = Vec::new();
    for (i, p) in pols.iter().enumerate() {
        let dir = join(base, p);
        let max_p = join(&dir, "scaling_max_freq");
        let gov_p = join(&dir, "scaling_governor");
        let cur_p = join(&dir, "scaling_cur_freq");
        let title = titles
            .get(i)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Cluster {}", i));
        if let (Some(max), Some(governor)) = (Handle::open(&max_p), Handle::open(&gov_p)) {
            out.push(Cluster {
                title,
                cur: Handle::open(&cur_p),
                max,
                governor,
            });
        }
    }
    out
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
