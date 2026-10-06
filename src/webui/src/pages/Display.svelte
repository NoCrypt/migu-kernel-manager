<script>
  import { onMount } from 'svelte';
  import PickerSheet from '../lib/components/PickerSheet.svelte';
  import Dialog from '../lib/components/Dialog.svelte';
  import ref1 from '../assets/color_cal_ref_1.webp';
  import ref2 from '../assets/color_cal_ref_2.webp';
  import ref3 from '../assets/color_cal_ref_3.webp';
  import { kmgr } from '../lib/backend.js';
  import { toast } from '../lib/ui.svelte.js';

  const samples = [ref1, ref2, ref3];
  let slide = $state(0);
  let track = $state(null);

  function onScroll() {
    if (!track) return;
    const i = Math.round(track.scrollLeft / track.clientWidth);
    if (i !== slide) slide = i;
  }

  function goSlide(i) {
    slide = i;
    if (track) track.scrollTo({ left: i * track.clientWidth, behavior: 'smooth' });
  }

  let entries = $state([]);
  let wm = $state(null);
  let vals = $state({});
  let picker = $state(null);
  let revert = $state(null);
  let loading = $state(true);
  let timer = null;
  const timers = {};

  async function load() {
    try {
      const t = JSON.parse(await kmgr('tree'));
      const g = (t.groups || []).find((x) => x.id === 'display');
      const kcal = (g?.sections || []).find((s) => s.id === 'kcal');
      entries = kcal?.entries || [];
      const v = {};
      for (const e of entries) v[e.id] = Number(e.value ?? 0);
      vals = v;
    } catch {
      entries = [];
    }
    await loadWm();
    loading = false;
  }

  async function loadWm() {
    try {
      wm = JSON.parse(await kmgr('wm'));
    } catch {
      wm = null;
    }
  }

  onMount(load);

  function labelClass(id) {
    return id === 'red' ? 'r' : id === 'green' ? 'g' : id === 'blue' ? 'b' : '';
  }

  function onSlide(entry, value) {
    vals = { ...vals, [entry.id]: Number(value) };
    clearTimeout(timers[entry.id]);
    timers[entry.id] = setTimeout(() => writeKcal(entry, value), 140);
  }

  async function writeKcal(entry, value) {
    const r = JSON.parse(await kmgr(`set ${entry.key} ${value}${entry.persisted ? ' --persist' : ''}`));
    if (!r.ok) {
      toast(r.error ? String(r.error) : 'Rejected');
    } else if (String(r.actual) !== String(value)) {
      vals = { ...vals, [entry.id]: Number(r.actual) };
      toast(`Clamped to ${r.actual}`);
    }
  }

  async function persistKcal(entry, on) {
    await kmgr(`${on ? 'persist' : 'unpersist'} ${entry.key}`);
    entries = entries.map((e) => (e.key === entry.key ? { ...e, persisted: on } : e));
  }

  async function resetKcal() {
    await kmgr('reset display.kcal');
    await load();
    toast('Colour calibration reset');
  }

  function openResolution() {
    picker = {
      label: 'Display resolution',
      type: 'string',
      value: wm?.size?.current || '',
      kind: 'size',
      help: 'Enter WxH, e.g. 1080x2400. Reverts in 15s unless you confirm.'
    };
  }

  function openDensity() {
    picker = {
      label: 'Display pixel density',
      type: 'int',
      value: String(wm?.density?.current ?? ''),
      min: 120,
      max: 1000,
      kind: 'density',
      help: 'Enter a density, e.g. 440. Reverts in 15s unless you confirm.'
    };
  }

  async function applyPicker(value) {
    const kind = picker.kind;
    const prev = wm;
    picker = null;
    const r = JSON.parse(await kmgr(`wm ${kind === 'size' ? 'size' : 'density'} ${value}`));
    if (!r.ok) {
      toast(r.error ? String(r.error) : 'Failed');
      return;
    }
    wm = r;
    startRevert(kind, prev);
  }

  function startRevert(kind, prev) {
    const label = kind === 'size' ? 'resolution' : 'density';
    const prevVal = kind === 'size' ? prev?.size?.override || null : prev?.density?.override || null;
    const doRevert = async () => {
      if (prevVal) await kmgr(`wm ${kind === 'size' ? 'size' : 'density'} ${prevVal}`);
      else await kmgr(`wm ${kind === 'size' ? 'size' : 'density'} reset`);
      await loadWm();
      toast(`Reverted ${label}`);
    };
    revert = { label, seconds: 15, revert: doRevert };
    clearInterval(timer);
    timer = setInterval(() => {
      if (!revert) {
        clearInterval(timer);
        return;
      }
      revert.seconds -= 1;
      if (revert.seconds <= 0) {
        clearInterval(timer);
        const r = revert;
        revert = null;
        r.revert();
      }
    }, 1000);
  }

  function keepDisplay() {
    clearInterval(timer);
    revert = null;
    toast('Display settings kept');
  }

  function revertNow() {
    if (!revert) return;
    clearInterval(timer);
    const r = revert;
    revert = null;
    r.revert();
  }

  async function resetDisplay() {
    await kmgr('wm reset');
    await loadWm();
    toast('Display settings reset');
  }
</script>

<div class="page">
  {#if loading}
    <p class="empty">Loading…</p>
  {:else}
    <div class="sample-card">
      <div class="sample-track" bind:this={track} onscroll={onScroll}>
        {#each samples as src, i (i)}
          <img src={src} alt={`Calibration sample ${i + 1}`} />
        {/each}
      </div>
    </div>
    <div class="pager">
      {#each samples as _, i (i)}
        <button class:on={slide === i} aria-label={`Sample ${i + 1}`} onclick={() => goSlide(i)}></button>
      {/each}
    </div>

    <h3 class="sec">Color calibration</h3>
    {#if entries.length === 0}
      <p class="empty">Colour calibration is not available on this kernel.</p>
    {:else}
      {#each entries as e (e.key)}
        <div class="slider-row">
          <div class="sl-main">
            <div class="sl-head">
              <span class="sl-label {labelClass(e.id)}">{e.label}</span>
              <span class="sl-val">{vals[e.id] ?? e.value}</span>
            </div>
            <input
              type="range"
              min={e.min}
              max={e.max}
              step={e.step || 1}
              value={vals[e.id] ?? e.value}
              oninput={(ev) => onSlide(e, ev.currentTarget.value)}
            />
          </div>
          <label class="sw">
            <input
              type="checkbox"
              checked={e.persisted}
              onchange={(ev) => persistKcal(e, ev.currentTarget.checked)}
            />
            <i></i>
          </label>
        </div>
      {/each}
      <div class="row">
        <button class="txt" onclick={resetKcal}>
          <div class="t">Reset colour calibration</div>
          <div class="s">Restore the first-seen values</div>
        </button>
      </div>
    {/if}

    <h3 class="sec">Display settings</h3>
    <div class="row">
      <button class="txt" onclick={openResolution}>
        <div class="t">Display resolution</div>
        <div class="s">Current: {wm?.size?.current || '—'}</div>
      </button>
    </div>
    <div class="row">
      <button class="txt" onclick={openDensity}>
        <div class="t">Display pixel density</div>
        <div class="s">Current: {wm?.density?.current ?? '—'} dp</div>
      </button>
    </div>
    <div class="row">
      <button class="txt" onclick={resetDisplay}>
        <div class="t">Reset display settings</div>
      </button>
    </div>
  {/if}
</div>

<PickerSheet entry={picker} onclose={() => (picker = null)} onselect={applyPicker} />

<Dialog
  open={!!revert}
  title="Keep display settings?"
  body={`${revert?.label ?? ''} reverts in ${revert?.seconds ?? 0}s unless you confirm.`}
  confirmLabel="Keep"
  onconfirm={keepDisplay}
  oncancel={revertNow}
/>
