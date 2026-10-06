<script>
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import { mountLive } from '../../live.js';
  import { prefs, setShowDeepSleep, setShowRareFreq } from '../../ui.svelte.js';
  import { mhz, shortDur, pctSmart } from '../../format.js';

  let rank = $state(0);
  let rows = $state([]);
  let deep = $state(null);
  let baseline = $state(null);
  let sinceReset = $state(false);
  let gear = $state(false);
  let deepBase = $state(0);

  const live = mountLive(
    () => `live cpustats --cluster ${rank} --interval 2000`,
    (s) => {
      rows = s.rows || [];
      deep = s.deep_sleep_ms ?? null;
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });

  function pickRank(r) {
    rank = r;
    baseline = null;
    sinceReset = false;
    deepBase = 0;
    live.restart();
  }

  let shown = $derived.by(() => {
    const src = rows.map((r) => ({
      khz: r.khz,
      ms: Math.max(0, r.ms - (baseline ? baseline[r.khz] || 0 : 0))
    }));
    const rawDeep = Math.max(0, (deep ?? 0) - (baseline ? deepBase : 0));
    const deepMs = prefs.showDeepSleep ? rawDeep : 0;
    const tot = src.reduce((a, r) => a + r.ms, 0) + deepMs;

    const list = src
      .filter((r) => r.ms > 0)
      .map((r) => ({ ...r, pct: tot ? (r.ms / tot) * 100 : 0 }));
    return { rows: list, total: tot, deepPct: tot ? (deepMs/tot)*100:0, deepMs };
  });

  let view = $derived(prefs.showRareFreq ? shown.rows : shown.rows.filter((r) => r.pct >= 0.5));

  function doReset() {
    const b = {};
    for (const r of rows) b[r.khz] = r.ms;
    baseline = b;
    sinceReset = true;
    gear = false;
    deepBase = deep ?? 0;
  }
</script>

<div class="cpustats-page">
  <div class="cpustats">
    {#if deep != null && prefs.showDeepSleep}
      <div class="rowbar">
        <div class="fill" style="width:{shown.deepPct || 0}%"></div>
        <div class="f1">Deep sleep</div>
        <div class="dur">{shortDur(shown.deepMs)}</div>
        <div class="pc">{pctSmart(shown.deepPct)}</div>
      </div>
    {/if}
    {#each view as r (r.khz)}
      <div class="rowbar">
        <div class="fill" style="width:{r.pct}%"></div>
        <div class="f1">{mhz(r.khz)}</div>
        <div class="dur">{shortDur(r.ms)}</div>
        <div class="pc">{pctSmart(r.pct)}</div>
      </div>
    {/each}
    {#if view.length === 0 && !(prefs.showDeepSleep && deep != null)}
      <p class="empty">No time-in-state data yet.</p>
    {/if}
  </div>

  <div class="cpustats-seg">
    {#each ['Little', 'Big', 'Prime'] as name, i}
      <button class="seg-btn" class:on={rank === i} onclick={() => pickRank(i)}>{name}</button>
    {/each}
    <button class="icon-btn" aria-label="Stats settings" onclick={() => (gear = true)}>
      <Icon name="settings" size={24} />
    </button>
  </div>
</div>

{#if gear}
  <div class="scrim" role="presentation" onclick={() => (gear = false)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>Time-in-state</h3>
    <p class="desc">Counters are cumulative since boot. Reset stores the current values as a baseline.</p>
    <div class="options">
      <button
        class="opt"
        class:on={!sinceReset}
        onclick={() => {
          sinceReset = false;
          baseline = null;
          gear = false;
        }}
      >
        Since boot
      </button>
      <button class="opt" class:on={sinceReset} onclick={doReset}>Reset stats</button>
    </div>
    <div class="row no-switch" style="padding-left:0;padding-right:0;">
      <div class="txt">
        <div class="t">Show rarely used frequencies</div>
        <div class="s">Frequencies below 0.5% of the time</div>
      </div>
      <label class="sw">
        <input
          type="checkbox"
          checked={prefs.showRareFreq}
          onchange={(e) => setShowRareFreq(e.currentTarget.checked)}
        />
        <i></i>
      </label>
    </div>
    <div class="row no-switch" style="padding-left:0;padding-right:0;">
      <div class="txt">
        <div class="t">Show Deep Sleep</div>
        <div class="s">Also add deep sleep to the stats.</div>
      </div>
      <label class="sw">
        <input
          type="checkbox"
          checked={prefs.showDeepSleep}
          onchange={(e) => setShowDeepSleep(e.currentTarget.checked)}
        />
        <i></i>
      </label>
    </div>
    <div class="actions">
      <button onclick={() => (gear = false)}>Close</button>
    </div>
  </div>
{/if}
