<script>
  import { onMount } from 'svelte';
  import PickerSheet from '../lib/components/PickerSheet.svelte';
  import { prefs, setShowHelp, setInterval, setAccentColor } from '../lib/ui.svelte.js';
  import { kmgr } from '../lib/backend.js';
  import { ACCENTS } from '../lib/accent.js';

  const intervalChoices = [
    { value: '500', label: '0.5 seconds' },
    { value: '1000', label: '1 second' },
    { value: '2000', label: '2 seconds' },
    { value: '5000', label: '5 seconds' }
  ];

  let picker = $state(null);
  let info = $state(null);
  let accentOpen = $state(false);

  function pickAccent(v) {
    setAccentColor(v);
  }

  onMount(async () => {
    try {
      info = JSON.parse(await kmgr('info'));
    } catch {
      info = null;
    }
  });

  function intervalLabel(v) {
    return intervalChoices.find((c) => c.value === String(v))?.label || `${v} ms`;
  }

  function openInterval() {
    picker = {
      label: 'Refresh interval',
      type: 'enum',
      choices: intervalChoices,
      value: String(prefs.interval)
    };
  }

  function choose(value) {
    setInterval(value);
    picker = null;
  }
</script>

<h3 class="sec">App settings</h3>

<div class="row">
  <div class="txt">
    <div class="t">Show descriptions</div>
    <div class="s">Show the hint text under each setting</div>
  </div>
  <label class="sw">
    <input type="checkbox" checked={prefs.showHelp} onchange={(e) => setShowHelp(e.currentTarget.checked)} />
    <i></i>
  </label>
</div>

<div class="row">
  <button class="txt" onclick={openInterval}>
    <div class="t">Refresh interval</div>
    <div class="s">{intervalLabel(prefs.interval)}</div>
  </button>
</div>

<div class="row">
  <button class="txt" onclick={() => (accentOpen = true)}>
    <div class="t">Accent color</div>
    <div class="s">{prefs.accent}</div>
  </button>
  <span class="swatch" style="background:{prefs.accent}"></span>
</div>

<section class="card" style="margin-top:24px">
  <span class="chip">About</span>
  <div class="line"><b>Module:</b> Migu Kernel Manager (kmgr)</div>
  {#if info?.model}
    <div class="line"><b>Device:</b> {info.model} ({info.device})</div>
  {/if}
  {#if info?.android}
    <div class="line"><b>Android:</b> {info.android}</div>
  {/if}
  {#if info?.kernel}
    <div class="line"><b>Kernel:</b> {info.kernel}</div>
  {/if}
</section>

<PickerSheet entry={picker} onclose={() => (picker = null)} onselect={choose} />

{#if accentOpen}
  <div class="scrim" role="presentation" onclick={() => (accentOpen = false)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>Accent color</h3>
    <p class="desc">Used for highlights, switches, chips and charts across the app.</p>
    <div class="swatches">
      {#each ACCENTS as c (c.value)}
        <button
          class="swatch-btn"
          class:on={prefs.accent.toLowerCase() === c.value}
          style="background:{c.value}"
          aria-label={c.name}
          title={c.name}
          onclick={() => pickAccent(c.value)}
        ></button>
      {/each}
    </div>
    <div class="row no-switch" style="padding-left:0;padding-right:0;">
      <div class="txt">
        <div class="t">Custom color</div>
        <div class="s">Pick any color</div>
      </div>
      <input type="color" value={prefs.accent} oninput={(e) => pickAccent(e.currentTarget.value)} />
    </div>
    <div class="actions">
      <button onclick={() => (accentOpen = false)}>Done</button>
    </div>
  </div>
{/if}
