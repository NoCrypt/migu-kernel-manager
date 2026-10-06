// `kmgr live <tab>` samplers: one NDJSON line per tick, running until killed
// (or `--seconds`, the WebUI's leak guard). See LIVE_MONITOR_AGENT.md.
use crate::monitor::Handle;
use crate::registry;
use crate::util::{join, list_dir, path_exists, read_str, rooted, trailing_number};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub fn run(args: &[String]) -> i32 {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "list" => {
            println!("{}", available());
            0
        }
        "processes" => processes(&args[1..]),
        "cpu" => cpu(&args[1..]),
        "cpustats" => cpustats(&args[1..]),
        "gpu" => gpu(&args[1..]),
        "ram" => ram(&args[1..]),
        "zram" => zram(&args[1..]),
        "ddr" => ddr(&args[1..]),
        "io" => io(&args[1..]),
        "wakelocks" => wakelocks(&args[1..]),
        "thermal" => thermal(&args[1..]),
        _ => {
            println!("{}", json!({"ok": false, "error": "usage: kmgr live <tab>", "code": 2}));
            2
        }
    }
}

struct Opts {
    interval: u64,
    seconds: Option<u64>,
    max: usize,
    cluster: usize,
    since: u64,
    follow: bool,
    export: bool,
}

fn parse(args: &[String]) -> Opts {
    let mut o = Opts {
        interval: 1000,
        seconds: None,
        max: 25,
        cluster: 0,
        since: 0,
        follow: false,
        export: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--interval" if i + 1 < args.len() => {
                o.interval = args[i + 1].parse().unwrap_or(1000);
                i += 2;
            }
            "--seconds" if i + 1 < args.len() => {
                o.seconds = args[i + 1].parse().ok();
                i += 2;
            }
            "--max" if i + 1 < args.len() => {
                o.max = args[i + 1].parse().unwrap_or(25);
                i += 2;
            }
            "--cluster" if i + 1 < args.len() => {
                o.cluster = args[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "--since" if i + 1 < args.len() => {
                o.since = args[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "--follow" => {
                o.follow = true;
                i += 1;
            }
            "--export" => {
                o.export = true;
                i += 1;
            }
            _ => i += 1,
        }
    }
    o
}

fn emit(lock: &mut impl Write, v: &Value) -> bool {
    serde_json::to_writer(&mut *lock, v).is_ok()
        && writeln!(lock).is_ok()
        && lock.flush().is_ok()
}

fn gone() -> i32 {
    println!("{}", json!({"gone": true}));
    0
}

fn stream<F>(opts: &Opts, mut tick: F) -> i32
where
    F: FnMut(u64) -> Value,
{
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let started = Instant::now();
    let mut t: u64 = 0;
    loop {
        t += 1;
        let v = tick(t);
        if !emit(&mut lock, &v) {
            return 0;
        }
        if let Some(sec) = opts.seconds {
            if started.elapsed().as_secs() >= sec {
                return 0;
            }
        }
        sleep(Duration::from_millis(opts.interval.max(200)));
    }
}

// ---------------------------------------------------------------- availability

pub fn available() -> Value {
    let mut tabs = Vec::new();
    tabs.push(tab("processes", "PROCESSES"));
    if path_exists("/proc/stat") {
        tabs.push(tab("cpu", "CPU"));
    }
    if has_cpustats() {
        tabs.push(tab("cpustats", "CPU STATS"));
    }
    if gpu_sources().is_some() {
        tabs.push(tab("gpu", "GPU"));
    }
    if path_exists("/proc/meminfo") {
        tabs.push(tab("ram", "RAM"));
    }
    if path_exists("/sys/block/zram0/disksize") {
        tabs.push(tab("zram", "ZRAM"));
    }
    if find_ddr().is_some() {
        tabs.push(tab("ddr", "DDR BUS"));
    }
    if path_exists("/proc/diskstats") {
        tabs.push(tab("io", "I/O"));
    }
    if has_wakelocks() {
        tabs.push(tab("wakelocks", "WAKELOCKS"));
    }
    if has_thermal() {
        tabs.push(tab("thermal", "THERMAL ZONES"));
    }
    if path_exists("/dev/kmsg") {
        tabs.push(tab("kernellog", "KERNEL LOG"));
    }
    json!({"ok": true, "tabs": tabs})
}

fn tab(id: &str, label: &str) -> Value {
    json!({"id": id, "label": label})
}

// ---------------------------------------------------------------------- ram

fn ram(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut h = match Handle::open("/proc/meminfo") {
        Some(h) => h,
        None => return gone(),
    };
    stream(&opts, |t| {
        let s = h.read().unwrap_or_default();
        let total = crate::monitor::meminfo_field(&s, "MemTotal").unwrap_or(0.0);
        let free = crate::monitor::meminfo_field(&s, "MemAvailable")
            .or_else(|| crate::monitor::meminfo_field(&s, "MemFree"))
            .unwrap_or(0.0);
        let used = (total - free).max(0.0);
        let usage = if total > 0.0 {
            (used / total * 100.0).round()
        } else {
            0.0
        };
        json!({"t": t, "total_kb": total, "free_kb": free, "used_kb": used, "usage": usage})
    })
}

// --------------------------------------------------------------------- zram

fn zram(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut disk = match Handle::open("/sys/block/zram0/disksize") {
        Some(h) => h,
        None => return gone(),
    };
    let mut mm = match Handle::open("/sys/block/zram0/mm_stat") {
        Some(h) => h,
        None => return gone(),
    };
    stream(&opts, |t| {
        let total = disk.read().and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0);
        let used = mm
            .read()
            .and_then(|s| s.split_whitespace().next().and_then(|x| x.parse::<f64>().ok()))
            .unwrap_or(0.0);
        let usage = if total > 0.0 {
            (used / total * 100.0).round()
        } else {
            0.0
        };
        json!({"t": t, "total_b": total, "used_b": used, "usage": usage})
    })
}

// ---------------------------------------------------------------------- cpu

fn policy_dirs() -> Vec<String> {
    let base = "/sys/devices/system/cpu/cpufreq";
    let mut pols: Vec<String> = list_dir(base)
        .into_iter()
        .filter(|n| n.starts_with("policy"))
        .collect();
    pols.sort_by_key(|n| trailing_number(n));
    pols
}

fn cpu_cluster_map() -> BTreeMap<usize, String> {
    let names = ["Little", "Big", "Prime"];
    let base = "/sys/devices/system/cpu/cpufreq";
    let mut m = BTreeMap::new();
    for (i, p) in policy_dirs().iter().enumerate() {
        let name = names.get(i).copied().unwrap_or("CPU").to_string();
        let dir = join(base, p);
        let list = read_str(&join(&dir, "related_cpus"))
            .or_else(|| read_str(&join(&dir, "affected_cpus")))
            .unwrap_or_default();
        for tok in list.split_whitespace() {
            if let Ok(n) = tok.parse::<usize>() {
                m.insert(n, name.clone());
            }
        }
    }
    m
}

fn present_cpus() -> Vec<usize> {
    if let Some(s) = read_str("/sys/devices/system/cpu/present") {
        let mut out = Vec::new();
        for part in s.split(',') {
            if let Some((a, b)) = part.split_once('-') {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                    for n in a..=b {
                        out.push(n);
                    }
                }
            } else if let Ok(n) = part.trim().parse::<usize>() {
                out.push(n);
            }
        }
        if !out.is_empty() {
            return out;
        }
    }
    let mut out: Vec<usize> = list_dir("/sys/devices/system/cpu")
        .into_iter()
        .filter(|n| n.starts_with("cpu") && n[3..].chars().all(|c| c.is_ascii_digit()))
        .filter_map(|n| n[3..].parse::<usize>().ok())
        .collect();
    out.sort_unstable();
    out
}

struct CoreFreq {
    cpu: usize,
    cur: Option<Handle>,
    online: Option<Handle>,
}

fn cpu(args: &[String]) -> i32 {
    let opts = parse(args);
    let clusters = cpu_cluster_map();
    let mut stat = match Handle::open("/proc/stat") {
        Some(h) => h,
        None => return gone(),
    };
    let mut cores: Vec<CoreFreq> = present_cpus()
        .into_iter()
        .map(|n| {
            let dir = format!("/sys/devices/system/cpu/cpu{}", n);
            CoreFreq {
                cpu: n,
                cur: Handle::open(&join(&dir, "cpufreq/scaling_cur_freq"))
                    .or_else(|| Handle::open(&join(&dir, "cpufreq/cpuinfo_cur_freq"))),
                online: Handle::open(&join(&dir, "online")),
            }
        })
        .collect();
    let (mut prev_overall, mut prev) = stat
        .read()
        .map(|s| crate::monitor::parse_stat(&s))
        .unwrap_or_default();
    stream(&opts, |t| {
        let (now_overall, now) = stat
            .read()
            .map(|s| crate::monitor::parse_stat(&s))
            .unwrap_or_default();
        let load = crate::monitor::busy_pct(prev_overall, now_overall)
            .map(|v| (v * 10.0).round() / 10.0);
        let mut arr: Vec<Value> = Vec::new();
        for c in cores.iter_mut() {
            let on = c
                .online
                .as_mut()
                .and_then(|h| h.read())
                .map(|v| v.trim() != "0")
                .unwrap_or(true);
            let khz = if on {
                c.cur
                    .as_mut()
                    .and_then(|h| h.read())
                    .and_then(|v| v.trim().parse::<i64>().ok())
                    .filter(|v| *v > 0)
            } else {
                None
            };
            let load = match now.get(&c.cpu).zip(prev.get(&c.cpu)) {
                Some((n, p)) => crate::monitor::busy_pct(*p, *n),
                None => None,
            };
            arr.push(json!({
                "i": c.cpu,
                "on": on,
                "load": load.map(|v| (v * 10.0).round() / 10.0),
                "khz": khz,
                "cluster": clusters.get(&c.cpu).cloned().unwrap_or_else(|| "CPU".into()),
            }));
        }
        prev_overall = now_overall;
        prev = now;
        json!({"t": t, "load": load, "cores": arr})
    })
}

// ----------------------------------------------------------------- cpustats

fn cpustats_path(rank: usize) -> Option<String> {
    let pols = policy_dirs();
    let p = pols.get(rank)?;
    Some(join(
        &join("/sys/devices/system/cpu/cpufreq", p),
        "stats/time_in_state",
    ))
}

fn has_cpustats() -> bool {
    policy_dirs()
        .iter()
        .any(|p| path_exists(&join(&join("/sys/devices/system/cpu/cpufreq", p), "stats/time_in_state")))
}

fn cpustats(args: &[String]) -> i32 {
    let opts = parse(args);
    let path = match cpustats_path(opts.cluster) {
        Some(p) => p,
        None => return gone(),
    };
    let mut h = match Handle::open(&path) {
        Some(h) => h,
        None => return gone(),
    };
    stream(&opts, |t| {
        let s = h.read().unwrap_or_default();
        let mut rows: Vec<Value> = Vec::new();
        let mut total = 0.0f64;
        for line in s.lines() {
            let mut it = line.split_whitespace();
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                if let (Ok(k), Ok(v)) = (k.parse::<i64>(), v.parse::<f64>()) {
                    let ms = v * 10.0;
                    total += ms;
                    rows.push(json!({"khz": k, "ms": ms}));
                }
            }
        }
        let (deep_ms, deep_pct) = match crate::monitor::deep_sleep() {
            Some((ms, pct)) => (Some(ms), Some(pct)),
            None => (None, None),
        };
        json!({"t": t, "rows": rows, "total_ms": total, "deep_sleep_ms": deep_ms, "deep_sleep_pct": deep_pct})
    })
}

// ---------------------------------------------------------------------- gpu

struct Gpu {
    busy: Option<Handle>,
    cur: Option<Handle>,
    max: Option<Handle>,
    min: Option<Handle>,
    min_fixed: Option<f64>,
}

fn gpu_sources() -> Option<Gpu> {
    let reg = registry::resolve();
    let src = reg.resolved_sources();
    let get = |k: &str| src.get(k).and_then(|p| Handle::open(p));
    let busy = get("gpu_busy");
    let cur = get("gpu_cur_freq");
    let max = get("gpu_max_freq");
    let min = get("gpu_min_freq");
    let mut min_fixed = None;
    if min.is_none() {
        if let Some(p) = src.get("gpu_available_frequencies") {
            min_fixed = read_str(p)
                .and_then(|s| s.split_whitespace().filter_map(|x| x.parse::<i64>().ok()).min())
                .map(|v| v as f64);
        }
    }
    if busy.is_none() && cur.is_none() {
        return None;
    }
    Some(Gpu { busy, cur, max, min, min_fixed })
}

fn gpu(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut g = match gpu_sources() {
        Some(g) => g,
        None => return gone(),
    };
    stream(&opts, |t| {
        let usage = g
            .busy
            .as_mut()
            .and_then(|h| h.read())
            .and_then(|v| crate::monitor::first_num(&v));
        let cur = g.cur.as_mut().and_then(|h| h.read()).and_then(|v| v.trim().parse::<i64>().ok());
        let max = g.max.as_mut().and_then(|h| h.read()).and_then(|v| v.trim().parse::<i64>().ok());
        let min = g
            .min
            .as_mut()
            .and_then(|h| h.read())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .or(g.min_fixed);
        json!({"t": t, "usage": usage, "cur_hz": cur, "min_hz": min, "max_hz": max})
    })
}

// ---------------------------------------------------------------------- ddr

struct Dev {
    cur: Handle,
    min: Option<Handle>,
    max: Option<Handle>,
}

fn find_ddr() -> Option<Dev> {
    let base = "/sys/class/devfreq";
    for name in list_dir(base) {
        let low = name.to_lowercase();
        let dev_target = std::fs::read_link(rooted(&join(&join(base, &name), "device")))
            .ok()
            .map(|p| p.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let hit = low.contains("ddr")
            || low.contains("llcc-ddr")
            || low.contains("bus-dcvs")
            || dev_target.contains("ddr")
            || dev_target.contains("llcc-ddr")
            || dev_target.contains("bus-dcvs");
        if !hit {
            continue;
        }
        let dir = join(base, &name);
        if !path_exists(&join(&dir, "cur_freq")) {
            continue;
        }
        if let Some(cur) = Handle::open(&join(&dir, "cur_freq")) {
            return Some(Dev {
                cur,
                min: Handle::open(&join(&dir, "min_freq")),
                max: Handle::open(&join(&dir, "max_freq")),
            });
        }
    }
    None
}

fn ddr(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut d = match find_ddr() {
        Some(d) => d,
        None => return gone(),
    };
    stream(&opts, |t| {
        let cur = d.cur.read().and_then(|v| v.trim().parse::<f64>().ok());
        let min = d.min.as_mut().and_then(|h| h.read()).and_then(|v| v.trim().parse::<f64>().ok());
        let max = d.max.as_mut().and_then(|h| h.read()).and_then(|v| v.trim().parse::<f64>().ok());
        let usage = match (cur, min, max) {
            (Some(c), Some(lo), Some(hi)) if hi > lo => ((c - lo) / (hi - lo) * 100.0).clamp(0.0, 100.0),
            _ => 0.0,
        };
        json!({"t": t, "usage": usage.round(), "cur_hz": cur, "min_hz": min, "max_hz": max})
    })
}

// ----------------------------------------------------------------------- io

#[derive(Default, Clone, Copy)]
struct Io {
    read: u64,
    write: u64,
    ticks: u64,
}

fn read_diskstats(s: &str) -> Io {
    // The physical UFS may be reported as sd[a-z], but on many Android kernels
    // those counters stay zero and the I/O shows up on the dm-* mapped
    // devices instead. Sum sd (whole disks) and dm; prefer sd when it has any
    // activity so a device that reports both is not double counted.
    let mut sd = Io::default();
    let mut dm = Io::default();
    for line in s.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 14 {
            continue;
        }
        let name = f[2];
        let b = name.as_bytes();
        let is_sd = b.len() == 3 && name.starts_with("sd") && b[2].is_ascii_alphabetic();
        let is_dm = name.starts_with("dm-") && name[3..].chars().all(|c| c.is_ascii_digit());
        if !is_sd && !is_dm {
            continue;
        }
        let read = f[5].parse::<u64>().unwrap_or(0);
        let write = f[9].parse::<u64>().unwrap_or(0);
        let ticks = f[12].parse::<u64>().unwrap_or(0);
        if is_sd {
            sd.read += read;
            sd.write += write;
            sd.ticks += ticks;
        } else {
            dm.read += read;
            dm.write += write;
            // dm volumes overlap in time; the busiest one approximates bus
            // utilisation far better than their sum.
            dm.ticks = dm.ticks.max(ticks);
        }
    }
    if sd.read + sd.write + sd.ticks > 0 {
        sd
    } else {
        dm
    }
}

fn io(args: &[String]) -> i32 {
    let opts = parse(args);
    let interval_s = opts.interval.max(200) as f64 / 1000.0;
    let mut h = match Handle::open("/proc/diskstats") {
        Some(h) => h,
        None => return gone(),
    };
    let mut prev = read_diskstats(&h.read().unwrap_or_default());
    stream(&opts, |t| {
        let now = read_diskstats(&h.read().unwrap_or_default());
        let dr = now.read.saturating_sub(prev.read) as f64 * 512.0 / interval_s;
        let dw = now.write.saturating_sub(prev.write) as f64 * 512.0 / interval_s;
        let dt = now.ticks.saturating_sub(prev.ticks) as f64;
        let usage = (dt / (interval_s * 1000.0) * 100.0).clamp(0.0, 100.0);
        prev = now;
        json!({"t": t, "usage": (usage * 10.0).round() / 10.0, "read_bps": dr, "write_bps": dw})
    })
}

// ---------------------------------------------------------------- wakelocks

fn has_wakelocks() -> bool {
    if list_dir("/sys/class/wakeup").iter().any(|d| d.starts_with("wakeup")) {
        return true;
    }
    path_exists("/sys/kernel/debug/wakeup_sources")
}

fn parse_duration_ms(s: &str) -> Option<f64> {
    let s = s.trim().trim_start_matches('-');
    if s.is_empty() {
        return None;
    }
    let b = s.as_bytes();
    let mut total = 0.0f64;
    let mut num = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_digit() {
            num.push(c);
            i += 1;
            continue;
        }
        let v: f64 = num.parse().ok()?;
        num.clear();
        match c {
            'd' => {
                total += v * 86_400_000.0;
                i += 1;
            }
            'h' => {
                total += v * 3_600_000.0;
                i += 1;
            }
            'm' if i + 1 < b.len() && b[i + 1] == b's' => {
                total += v;
                i += 2;
            }
            'm' => {
                total += v * 60_000.0;
                i += 1;
            }
            's' => {
                total += v * 1_000.0;
                i += 1;
            }
            _ => i += 1,
        }
    }
    Some(total)
}

fn parse_pm_line(line: &str) -> Option<(String, f64)> {
    let start = line.find('\'')?;
    let rest = &line[start + 1..];
    let end = rest.find('\'')?;
    let name = rest[..end].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let acq = line.find("ACQ=")?;
    let val: String = line[acq + 4..]
        .chars()
        .take_while(|c| *c != ' ' && *c != '(')
        .collect();
    let ms = parse_duration_ms(&val)?;
    Some((name, ms))
}

// Android PowerManager wake locks from `dumpsys power` (userspace locks such
// as app-held FULL_WAKE_LOCKs, which are not kernel wakeup sources).
fn pm_wakelocks() -> Vec<(String, f64)> {
    let out = std::process::Command::new("/system/bin/dumpsys")
        .arg("power")
        .output()
        .or_else(|_| std::process::Command::new("dumpsys").arg("power").output());
    let out = match out {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut res = Vec::new();
    let mut in_sec = false;
    for line in text.lines() {
        if line.contains("Wake Locks:") {
            in_sec = true;
            continue;
        }
        if !in_sec {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(item) = parse_pm_line(line) {
                res.push(item);
            }
        } else if !line.trim().is_empty() {
            break;
        }
    }
    res.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    res
}

fn sort_truncate(mut arr: Vec<Value>) -> Vec<Value> {
    arr.sort_by(|a, b| {
        b["ms"].as_f64().unwrap_or(0.0).partial_cmp(&a["ms"].as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal)
    });
    arr.truncate(100);
    arr
}

fn read_int(h: Option<&mut Handle>) -> Option<i64> {
    h.and_then(|x| x.read()).and_then(|v| v.trim().parse::<i64>().ok())
}

fn read_f(h: Option<&mut Handle>) -> Option<f64> {
    h.and_then(|x| x.read()).and_then(|v| v.trim().parse::<f64>().ok())
}

struct Wl {
    name: Handle,
    total: Handle,
    active: Option<Handle>,
    events: Option<Handle>,
    wakeup: Option<Handle>,
    expire: Option<Handle>,
    max: Option<Handle>,
}

fn open_wl(dir: &str) -> Option<Wl> {
    Some(Wl {
        name: Handle::open(&join(dir, "name"))?,
        total: Handle::open(&join(dir, "total_time_ms"))?,
        active: Handle::open(&join(dir, "active_count")),
        events: Handle::open(&join(dir, "event_count")),
        wakeup: Handle::open(&join(dir, "wakeup_count")),
        expire: Handle::open(&join(dir, "expire_count")),
        max: Handle::open(&join(dir, "max_time_ms")),
    })
}

fn wakelocks(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut items: Vec<Wl> = list_dir("/sys/class/wakeup")
        .iter()
        .filter(|d| d.starts_with("wakeup"))
        .filter_map(|d| open_wl(&join("/sys/class/wakeup", d)))
        .collect();
    let mut dbg = if items.is_empty() {
        Handle::open("/sys/kernel/debug/wakeup_sources")
    } else {
        None
    };
    if items.is_empty() && dbg.is_none() {
        return gone();
    }
    let mut pm_cache: Vec<(String, f64)> = Vec::new();
    let mut pm_at: Option<Instant> = None;
    stream(&opts, |t| {
        let arr: Vec<Value> = if !items.is_empty() {
            items
                .iter_mut()
                .filter_map(|w| {
                    let name = w.name.read()?.trim().to_string();
                    if name.is_empty() {
                        return None;
                    }
                    let total = w.total.read()?.trim().parse::<f64>().ok()?;
                    if total <= 0.0 {
                        return None;
                    }
                    Some(json!({
                        "name": name,
                        "ms": total,
                        "active": read_int(w.active.as_mut()),
                        "events": read_int(w.events.as_mut()),
                        "wakeup": read_int(w.wakeup.as_mut()),
                        "expire": read_int(w.expire.as_mut()),
                        "max_ms": read_f(w.max.as_mut()),
                    }))
                })
                .collect()
        } else {
            let s = dbg.as_mut().and_then(|h| h.read()).unwrap_or_default();
            s.lines()
                .enumerate()
                .filter(|(i, line)| !(*i == 0 && line.contains("name")))
                .filter_map(|(_, line)| {
                    let f: Vec<&str> = line.split_whitespace().collect();
                    if f.len() < 8 {
                        return None;
                    }
                    let total = f[6].parse::<f64>().ok()?;
                    if total <= 0.0 {
                        return None;
                    }
                    Some(json!({
                        "name": f[0],
                        "ms": total,
                        "active": f[1].parse::<i64>().ok(),
                        "events": f[2].parse::<i64>().ok(),
                        "wakeup": f[3].parse::<i64>().ok(),
                        "expire": f[4].parse::<i64>().ok(),
                        "max_ms": f[7].parse::<f64>().ok(),
                    }))
                })
                .collect()
        };
        // Refresh the PowerManager list occasionally; between refreshes the
        // held locks keep counting up.
        let now = Instant::now();
        if pm_at.map_or(true, |x| now.duration_since(x).as_secs() >= 5) {
            pm_cache = pm_wakelocks();
            pm_at = Some(now);
        }
        let extra = pm_at.map(|x| now.duration_since(x).as_millis() as f64).unwrap_or(0.0);
        // dumpsys lists only currently-held locks: one active instance, and
        // PowerManager does not track suspend-abort counts (0).
        let pm: Vec<Value> = pm_cache
            .iter()
            .map(|(n, ms)| json!({"name": n, "ms": ms + extra, "active": 1, "wakeup": 0}))
            .collect();
        json!({"t": t, "pm": pm, "sources": sort_truncate(arr)})
    })
}

// ---------------------------------------------------------------- thermal

fn has_thermal() -> bool {
    list_dir("/sys/class/thermal")
        .iter()
        .any(|d| d.starts_with("thermal_zone") && path_exists(&join(&join("/sys/class/thermal", d), "temp")))
}

fn thermal(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut zones: Vec<(i64, String, Handle)> = Vec::new();
    for d in list_dir("/sys/class/thermal") {
        if !d.starts_with("thermal_zone") {
            continue;
        }
        let idx = trailing_number(&d);
        let dir = join("/sys/class/thermal", &d);
        let typ = read_str(&join(&dir, "type")).unwrap_or_default();
        // Skip battery current/voltage *level* zones (e.g. pm8150b-ibat-lvl0):
        // they read as raw values, not temperatures.
        if typ.contains("lvl") {
            continue;
        }
        if let Some(temp) = Handle::open(&join(&dir, "temp")) {
            zones.push((idx, typ, temp));
        }
    }
    zones.sort_by_key(|z| z.0);
    if zones.is_empty() {
        return gone();
    }
    stream(&opts, |t| {
        let mut arr: Vec<Value> = Vec::new();
        for (i, name, temp) in zones.iter_mut() {
            if let Some(v) = temp.read().and_then(|v| v.trim().parse::<f64>().ok()) {
                let c = if v >= 200.0 { v / 1000.0 } else { v };
                arr.push(json!({"i": i, "type": name, "c": (c * 10.0).round() / 10.0}));
            }
        }
        json!({"t": t, "zones": arr})
    })
}

// ---------------------------------------------------------------- processes

fn read_proc_stat(pid: i64) -> Option<(String, u64)> {
    let s = std::fs::read_to_string(rooted(&format!("/proc/{}/stat", pid))).ok()?;
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let comm = s[open + 1..close].to_string();
    let rest = &s[close + 1..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    let utime = f.get(11)?.parse::<u64>().ok()?;
    let stime = f.get(12)?.parse::<u64>().ok()?;
    Some((comm, utime + stime))
}

fn rss_mb(pid: i64) -> f64 {
    if let Ok(s) = std::fs::read_to_string(rooted(&format!("/proc/{}/statm", pid))) {
        if let Some(pages) = s.split_whitespace().nth(1).and_then(|x| x.parse::<u64>().ok()) {
            return pages as f64 * 4096.0 / 1048576.0;
        }
    }
    0.0
}

fn processes(args: &[String]) -> i32 {
    let opts = parse(args);
    let mut stat = match Handle::open("/proc/stat") {
        Some(h) => h,
        None => return gone(),
    };
    let mut prev: BTreeMap<i64, u64> = BTreeMap::new();
    let mut prev_total: u64 = 0;
    let mut first = true;
    stream(&opts, |t| {
        let s = stat.read().unwrap_or_default();
        let total: u64 = s
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("cpu "))
            .map(|rest| rest.split_whitespace().filter_map(|x| x.parse::<u64>().ok()).sum())
            .unwrap_or(0);
        let mut cur: BTreeMap<i64, (String, u64)> = BTreeMap::new();
        if let Ok(rd) = std::fs::read_dir(rooted("/proc")) {
            for e in rd.flatten() {
                let ns = e.file_name().to_string_lossy().into_owned();
                if let Ok(pid) = ns.parse::<i64>() {
                    if let Some((comm, ticks)) = read_proc_stat(pid) {
                        cur.insert(pid, (comm, ticks));
                    }
                }
            }
        }
        let mut list: Vec<(f64, String, i64)> = Vec::new();
        if !first {
            let dt = total.saturating_sub(prev_total);
            if dt > 0 {
                for (pid, (comm, ticks)) in &cur {
                    if let Some(pt) = prev.get(pid) {
                        let d = ticks.saturating_sub(*pt);
                        if d > 0 {
                            list.push((d as f64 * 100.0 / dt as f64, comm.clone(), *pid));
                        }
                    }
                }
            }
        }
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        list.truncate(opts.max.max(1));
        let arr: Vec<Value> = list
            .iter()
            .map(|(cpu, comm, pid)| {
                json!({
                    "pid": pid,
                    "name": comm,
                    "cpu": (cpu * 10.0).round() / 10.0,
                    "rss_mb": (rss_mb(*pid) * 10.0).round() / 10.0,
                })
            })
            .collect();
        prev = cur.iter().map(|(p, (_, t))| (*p, *t)).collect();
        prev_total = total;
        first = false;
        json!({"t": t, "procs": arr})
    })
}

#[cfg(unix)]
fn send_kill(pid: i64) -> std::io::Result<()> {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    let rc = unsafe { kill(pid as i32, 9) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn send_kill(_pid: i64) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "kill is not supported on this platform",
    ))
}

// One-shot: kill a process by pid (SIGKILL). The pid is parsed as an integer and
// handed straight to kill(2); no shell is involved.
pub fn kill_process(args: &[String]) -> i32 {
    let pid: i64 = match args.first().and_then(|s| s.trim().parse::<i64>().ok()) {
        Some(p) => p,
        None => {
            println!("{}", json!({"ok": false, "error": "usage: kmgr kill <pid>"}));
            return 2;
        }
    };
    if pid <= 1 {
        println!("{}", json!({"ok": false, "pid": pid, "error": "refusing to kill pid <= 1"}));
        return 1;
    }
    if pid == std::process::id() as i64 {
        println!("{}", json!({"ok": false, "pid": pid, "error": "refusing to kill kmgr itself"}));
        return 1;
    }
    let name = read_proc_stat(pid).map(|(c, _)| c).unwrap_or_default();
    match send_kill(pid) {
        Ok(()) => {
            println!("{}", json!({"ok": true, "pid": pid, "name": name, "signal": 9}));
            0
        }
        Err(e) => {
            println!("{}", json!({"ok": false, "pid": pid, "name": name, "error": e.to_string()}));
            1
        }
    }
}

// ------------------------------------------------------------------ dmesg

#[cfg(unix)]
fn open_kmsg() -> Option<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0o4000) // O_NONBLOCK
        .open(rooted("/dev/kmsg"))
        .ok()
}

#[cfg(not(unix))]
fn open_kmsg() -> Option<std::fs::File> {
    None
}

fn read_kmsg(file: &mut std::fs::File) -> Option<(u64, String)> {
    use std::io::Read;
    let mut buf = [0u8; 8192];
    match file.read(&mut buf) {
        Ok(0) => None,
        Ok(n) => parse_kmsg(&String::from_utf8_lossy(&buf[..n])),
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
        Err(_) => None,
    }
}

fn parse_kmsg(raw: &str) -> Option<(u64, String)> {
    let raw = raw.trim_end_matches(['\n', '\r']);
    let semi = raw.find(';')?;
    let header = &raw[..semi];
    let msg = &raw[semi + 1..];
    let parts: Vec<&str> = header.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let seq: u64 = parts[1].parse().ok()?;
    let usec: f64 = parts[2].parse().ok()?;
    let secs = usec / 1_000_000.0;
    Some((seq, format!("[{:>12.6}] {}", secs, msg)))
}

fn dmesg(args: &[String]) -> i32 {
    let opts = parse(args);
    if opts.export {
        return dmesg_export();
    }
    let mut file = match open_kmsg() {
        Some(f) => f,
        None => return gone(),
    };
    let out = std::io::stdout();
    let mut lock = out.lock();
    // Seed from the buffer, keeping the last 1000 records.
    let mut seed: Vec<(u64, String)> = Vec::new();
    while let Some(rec) = read_kmsg(&mut file) {
        if rec.0 > opts.since {
            seed.push(rec);
        }
    }
    let start = seed.len().saturating_sub(1000);
    let mut last = opts.since;
    for (seq, line) in &seed[start..] {
        if !emit(&mut lock, &json!({"n": seq, "t": line})) {
            return 0;
        }
        last = *seq;
    }
    if !opts.follow {
        return 0;
    }
    let started = Instant::now();
    loop {
        if let Some(sec) = opts.seconds {
            if started.elapsed().as_secs() >= sec {
                return 0;
            }
        }
        let mut got = false;
        while let Some((seq, line)) = read_kmsg(&mut file) {
            if seq > last {
                if !emit(&mut lock, &json!({"n": seq, "t": line})) {
                    return 0;
                }
                last = seq;
            }
            got = true;
        }
        if !got {
            sleep(Duration::from_millis(opts.interval.max(200)));
        }
    }
}

fn dmesg_export() -> i32 {
    let mut file = match open_kmsg() {
        Some(f) => f,
        None => return gone(),
    };
    let mut lines: Vec<String> = Vec::new();
    while let Some((_, line)) = read_kmsg(&mut file) {
        lines.push(line);
    }
    let dir = "/sdcard/Download";
    let _ = std::fs::create_dir_all(rooted(dir));
    let path = format!("/sdcard/Download/kernel_log_{}.txt", timestamp());
    match std::fs::write(rooted(&path), lines.join("\n")) {
        Ok(_) => {
            println!("{}", json!({"ok": true, "path": path}));
            0
        }
        Err(e) => {
            println!("{}", json!({"ok": false, "error": e.to_string()}));
            1
        }
    }
}

fn timestamp() -> String {
    std::process::Command::new("date")
        .arg("+%Y%m%d-%H%M%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "export".to_string())
}

pub fn dmesg_run(args: &[String]) -> i32 {
    dmesg(args)
}
