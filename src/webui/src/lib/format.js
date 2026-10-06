export const mhz = (khz) => (khz == null ? '—' : `${Math.trunc(khz / 1000)} MHz`);
export const hzToMhz = (h) => (h == null ? '—' : `${Math.trunc(h / 1e6)} MHz`);
export const mib = (m) => (m == null ? '—' : `${Math.round(m)} MiB`);
export const ma = (ua) => (ua == null ? '—' : `${Math.round(ua / 1000)} mA`);
export const volt = (uv) => (uv == null ? '—' : `${(uv / 1e6).toFixed(2)} V`);
export const celsius = (t) => (t == null ? '—' : `${Number(t).toFixed(1)} °C`);
export const pct = (v) => (v == null ? '—' : `${v}%`);

export function uptime(seconds) {
  if (seconds == null) return '—';
  const s = Math.floor(seconds);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m ${sec}s`;
  return `${m}m ${sec}s`;
}

export function health(chargeFull, design) {
  if (!chargeFull || !design) return '—';
  return `${Math.round((chargeFull / design) * 100)}%`;
}

export function watts(uv, ua) {
  if (uv == null || ua == null) return null;
  return `${(Math.abs(uv * ua) / 1e12).toFixed(1)} W`;
}

export function healthLabel(pct) {
  return pct >= 80 ? 'Good' : pct >= 60 ? 'Fair' : 'Poor';
}

export function chargeStatus(status, ua) {
  const s = String(status || '');
  const mA = Math.abs(ua || 0) / 1000;
  if (/discharg/i.test(s)) return 'Discharging';
  if (/full/i.test(s)) return 'Full';
  if (/not\s*charg/i.test(s)) return 'Not charging';
  if (/charg/i.test(s)) {
    if (mA < 500) return 'Charging slowly';
    if (mA >= 1500) return 'Charging rapidly';
    return 'Charging';
  }
  return s || '—';
}

export function isCharging(status) {
  const s = String(status || '');
  return /charg/i.test(s) && !/discharg/i.test(s) && !/not\s*charg/i.test(s);
}

export function tempC(t) {
  return t == null ? '—' : `${Math.trunc(t)}°C`;
}

export function durationMs(ms) {
  if (ms == null) return '—';
  const s = Math.floor(ms / 1000);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m ${sec}s`;
  return `${m}m ${sec}s`;
}

export function ratioPct(part, whole) {
  if (!whole) return '0%';
  return `${Math.round((part / whole) * 100)}%`;
}

export function ratioPct2(part, whole) {
  if (!whole) return '0.00%';
  return `${((part / whole) * 100).toFixed(2)}%`;
}

export function bytesToMib(b) {
  if (b == null) return '—';
  return `${Math.round(b / 1048576)} MiB`;
}

// Live Monitor helpers.
export const gib = (kb) => (kb == null ? '—' : `${(kb / 1048576).toFixed(1)}GB`);
export const gibFromBytes = (b) => (b == null ? '—' : `${(b / 1073741824).toFixed(1)}GB`);

// Devfreq bus nodes are inconsistent: some report Hz, some report the raw
// level (e.g. Qualcomm bandwidth votes). Normalise to a MHz-style number.
export const busSpeed = (v) =>
  v == null ? '—' : `${Math.trunc(v >= 1e6 ? v / 1e6 : v)} MHz`;

export function bps(v) {
  if (v == null) return '—';
  const u = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  let x = v;
  let i = 0;
  while (x >= 1024 && i < u.length - 1) {
    x /= 1024;
    i += 1;
  }
  const n = i === 0 ? Math.round(x) : x < 10 ? x.toFixed(1) : Math.round(x);
  return `${n} ${u[i]}`;
}

export function shortDur(ms) {
  if (ms == null) return '—';
  const s = Math.floor(ms / 1000);
  if (s < 1) return `${Math.round(ms)}ms`;
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

export function pctSmart(v) {
  if (v == null) return '—';
  return v < 10 ? `${v.toFixed(1)}%` : `${Math.round(v)}%`;
}
