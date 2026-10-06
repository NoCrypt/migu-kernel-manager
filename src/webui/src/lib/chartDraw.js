// Shared area-chart drawing for the Live Monitor (chart pages and CPU tiles).
export function cssVar(el, name, fallback) {
  const v = getComputedStyle(el).getPropertyValue(name).trim();
  return v || fallback;
}

// opts: { spacing } fixed px between samples, or { slots } to fit that many
// samples across the full width. Plus { lineWidth } and { fit } (with
// spacing, stretch the visible samples so they span edge to edge).
export function drawArea(canvas, values, opts = {}) {
  if (!canvas) return;
  const spacing = opts.spacing || 20;
  const slots = opts.slots || 0;
  const fit = opts.fit || false;
  const lineWidth = opts.lineWidth || 1;

  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  if (!w || !h) return;
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);
  const ctx = canvas.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  if (!values || !values.length) return;

  let visible;
  let step;
  if (slots > 0) {
    visible = slots;
    step = slots > 1 ? w / (slots - 1) : w;
  } else if (fit) {
    visible = Math.max(2, Math.round(w / spacing) + 1);
    step = w / (visible - 1);
  } else {
    visible = Math.max(1, Math.floor(w / spacing));
    step = spacing;
  }
  const samples = values.slice(-visible);
  const n = samples.length;
  const x0 = w - (n - 1) * step;
  const y = (val) => h - (Math.max(0, Math.min(100, val)) / 100) * h;

  ctx.beginPath();
  ctx.moveTo(x0, h);
  samples.forEach((val, i) => ctx.lineTo(x0 + i * step, y(val)));
  ctx.lineTo(x0 + (n - 1) * step, h);
  ctx.closePath();
  ctx.fillStyle = cssVar(canvas, '--chart-fill', '#07382c');
  ctx.fill();

  ctx.strokeStyle = cssVar(canvas, '--chart-line', '#64ffda');
  ctx.lineWidth = lineWidth;
  ctx.lineJoin = 'round';
  ctx.beginPath();
  samples.forEach((val, i) => (i ? ctx.lineTo(x0 + i * step, y(val)) : ctx.moveTo(x0, y(val))));
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(x0, y(samples[0]));
  ctx.lineTo(x0, h);
  ctx.stroke();
}
