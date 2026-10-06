// Shared UI state (Svelte 5 runes in a module).
import { kmgr } from './backend.js';
import { applyAccent, DEFAULT_ACCENT } from './accent.js';

export const ui = $state({
  route: 'dashboard',
  drawer: false,
  toasts: [],
  bootFailed: false
});

const DEFAULTS = {
  showHelp: true,
  interval: 1000,
  chartInterval: 1000,
  procRate: 2000,
  procMax: 25,
  logLines: true,
  showRareFreq: false,
  liveTab: 'cpustats',
  accent: DEFAULT_ACCENT,
  showDeepSleep: true
};

// App settings, persisted in /data/adb/kmgr/settings.json (via `kmgr settings`).
export const prefs = $state({ ...DEFAULTS });

function asBool(v, d) {
  if (v == null) return d;
  return v === true || v === '1' || v === 'true';
}
function asNum(v, d) {
  const n = Number(v);
  return Number.isFinite(n) && n > 0 ? n : d;
}

export function applySettings(s) {
  if (!s) return;
  prefs.showHelp = asBool(s.showHelp, DEFAULTS.showHelp);
  prefs.interval = asNum(s.interval, DEFAULTS.interval);
  prefs.chartInterval = asNum(s.chartInterval, DEFAULTS.chartInterval);
  prefs.procRate = asNum(s.procRate, DEFAULTS.procRate);
  prefs.procMax = asNum(s.procMax, DEFAULTS.procMax);
  prefs.logLines = asBool(s.logLines, DEFAULTS.logLines);
  prefs.showRareFreq = asBool(s.showRareFreq, DEFAULTS.showRareFreq);
  prefs.liveTab = s.liveTab || DEFAULTS.liveTab;
  prefs.accent = s.accent || DEFAULTS.accent;
  prefs.showDeepSleep = asBool(s.showDeepSleep, DEFAULTS.showDeepSleep);
}

export async function loadSettings() {
  try {
    const r = JSON.parse(await kmgr('settings'));
    applySettings(r.settings || {});
  } catch {
    /* keep defaults */
  }
}

function persist(key, value) {
  const v = String(value).replace(/[^0-9a-zA-Z#_.-]/g, '');
  kmgr(`settings set ${key} '${v}'`).catch(() => {});
}

export function setShowHelp(v) {
  prefs.showHelp = !!v;
  persist('showHelp', v ? '1' : '0');
}

export function setInterval(v) {
  prefs.interval = Number(v) || 1000;
  persist('interval', prefs.interval);
}

export function setChartInterval(v) {
  prefs.chartInterval = Number(v) || 1000;
  persist('chartInterval', prefs.chartInterval);
}

export function setProcRate(v) {
  prefs.procRate = Number(v) || 2000;
  persist('procRate', prefs.procRate);
}

export function setProcMax(v) {
  prefs.procMax = Number(v) || 25;
  persist('procMax', prefs.procMax);
}

export function setLogLines(v) {
  prefs.logLines = !!v;
  persist('logLines', v ? '1' : '0');
}

export function setShowRareFreq(v) {
  prefs.showRareFreq = !!v;
  persist('showRareFreq', v ? '1' : '0');
}

export function setShowDeepSleep(v) {
  prefs.showDeepSleep = !!v;
  persist('showDeepSleep', v ? '1' : '0');
}

export function setLiveTab(id) {
  prefs.liveTab = id;
  persist('liveTab', id);
}

export function setAccentColor(v) {
  prefs.accent = v;
  applyAccent(v);
  persist('accent', v);
}

export function navigate(route) {
  ui.route = route;
  ui.drawer = false;
  if (typeof location !== 'undefined') location.hash = `#/${route}`;
}

export function toast(message) {
  const id = Math.random().toString(36).slice(2);
  ui.toasts = [...ui.toasts, { id, message }];
  setTimeout(() => {
    ui.toasts = ui.toasts.filter((t) => t.id !== id);
  }, 2800);
}

export function routeFromHash() {
  if (typeof location === 'undefined') return 'dashboard';
  const h = location.hash.replace(/^#\/?/, '');
  return h || 'dashboard';
}
