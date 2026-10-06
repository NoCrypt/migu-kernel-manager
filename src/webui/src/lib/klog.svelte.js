// Kernel log state, shared between the Live Monitor shell (which renders the
// tab's action buttons in the bottom bar) and the list component.
import { mountLive } from './live.js';
import { prefs, setLogLines, toast } from './ui.svelte.js';
import { kmgr } from './backend.js';

const MAX = 5000;

export const klog = $state({
  lines: [],
  paused: false,
  baseline: 0,
  searchOpen: false,
  term: '',
  cur: 0,
  menu: false
});

let liveObj = null;
let sinceRef = { v: 0 };

function onLine(s) {
  if (s.n <= sinceRef.v) return;
  sinceRef.v = s.n;
  const next = [...klog.lines, { n: s.n, t: s.t }];
  klog.lines = next.length > MAX ? next.slice(-MAX) : next;
}

export function attach() {
  if (liveObj) return;
  liveObj = mountLive(
    () => `dmesg --follow --since ${sinceRef.v} --interval 500`,
    onLine
  );
  liveObj.attach();
}

export function detach() {
  if (!liveObj) return;
  liveObj.detach();
  liveObj = null;
  klog.paused = false;
}

export function togglePause() {
  klog.paused = !klog.paused;
  if (!liveObj) return;
  if (klog.paused) liveObj.halt();
  else liveObj.start();
}

export function openSearch() {
  klog.searchOpen = true;
  klog.cur = 0;
  if (!klog.paused) togglePause();
}

export function closeSearch() {
  klog.searchOpen = false;
  klog.term = '';
  if (klog.paused) togglePause();
}

export function setTerm(v) {
  klog.term = v;
  klog.cur = 0;
}

// Indices (into the visible list) of matching lines.
export function matchIndices() {
  if (!klog.term) return [];
  const q = klog.term.toLowerCase();
  const out = [];
  let i = 0;
  for (const l of klog.lines) {
    if (l.n <= klog.baseline) continue;
    if (l.t.toLowerCase().includes(q)) out.push(i);
    i += 1;
  }
  return out;
}

export function nextMatch(dir) {
  const n = matchIndices().length;
  if (!n) return;
  klog.cur = (klog.cur + dir + n) % n;
}

export function clearView() {
  klog.baseline = sinceRef.v;
  klog.menu = false;
}

export function toggleLineNumbers() {
  setLogLines(!prefs.logLines);
  klog.menu = false;
}

export async function exportLog() {
  klog.menu = false;
  const r = JSON.parse(await kmgr('dmesg --export'));
  toast(r.ok ? `Saved to ${r.path}` : r.error || 'Export failed');
}
