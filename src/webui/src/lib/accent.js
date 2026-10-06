// Custom accent colour. A single base hex is expanded into the theme's accent
// tokens at runtime, so the whole UI (switches, charts, chips, FAB) follows it.
export const DEFAULT_ACCENT = '#1de9b6';

export const ACCENTS = [
  { name: 'Teal', value: '#1de9b6' },
  { name: 'Mint', value: '#00e676' },
  { name: 'Green', value: '#4caf50' },
  { name: 'Blue', value: '#2196f3' },
  { name: 'Indigo', value: '#5c6bc0' },
  { name: 'Purple', value: '#ab47bc' },
  { name: 'Pink', value: '#ec407a' },
  { name: 'Red', value: '#ef5350' },
  { name: 'Orange', value: '#ff9800' },
  { name: 'Yellow', value: '#ffd600' }
];

function hexToRgb(hex) {
  let h = String(hex || '').replace('#', '');
  if (h.length === 3) h = h.split('').map((c) => c + c).join('');
  if (h.length !== 6 || /[^0-9a-f]/i.test(h)) return { r: 29, g: 233, b: 182 };
  const n = parseInt(h, 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

function toHex({ r, g, b }) {
  const f = (v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0');
  return `#${f(r)}${f(g)}${f(b)}`;
}

const mix = (a, b, t) => ({
  r: a.r * (1 - t) + b.r * t,
  g: a.g * (1 - t) + b.g * t,
  b: a.b * (1 - t) + b.b * t
});

const BLACK = { r: 0, g: 0, b: 0 };
const WHITE = { r: 255, g: 255, b: 255 };
const CARD = { r: 0x22, g: 0x22, b: 0x22 };

export function applyAccent(hex) {
  if (typeof document === 'undefined') return;
  const a = hexToRgb(hex);
  const lum = (0.299 * a.r + 0.587 * a.g + 0.114 * a.b) / 255;
  const vars = {
    '--accent': toHex(a),
    '--accent-mint': toHex(mix(a, WHITE, 0.35)),
    '--accent-dim': toHex(mix(a, BLACK, 0.76)),
    '--accent-pill': toHex(mix(a, CARD, 0.7)),
    '--sw-on-thumb': toHex(a),
    '--sw-on-track': toHex(mix(a, BLACK, 0.7)),
    '--on-accent': lum > 0.55 ? '#000000' : '#ffffff'
  };
  const root = document.documentElement;
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
}
