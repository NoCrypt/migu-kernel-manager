<script>
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import Dialog from '../Dialog.svelte';
  import { mountLive } from '../../live.js';
  import { prefs, setProcRate, setProcMax, toast } from '../../ui.svelte.js';
  import { kmgr } from '../../backend.js';
  import { pct } from '../../format.js';

  let procs = $state([]);
  let sheet = $state(false);
  let target = $state(null);
  let rate = $state(prefs.procRate);
  let max = $state(prefs.procMax);

  let sortKey = $state('cpu');
  let sortDir = $state('desc');

  const live = mountLive(
    () => `live processes --interval ${rate} --max ${max}`,
    (s) => {
      const list = s.procs || [];
      // The first tick has no deltas yet and a transient read can come back
      // empty; only replace the table when we actually got processes.
      if (list.length) procs = list;
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });

  function sortBy(k) {
    if (sortKey === k) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = k;
      sortDir = k === 'name' ? 'asc' : 'desc';
    }
  }

  const arrow = (k) => (sortKey === k ? (sortDir === 'asc' ? ' ▲' : ' ▼') : '');

  let sorted = $derived.by(() => {
    const arr = [...procs];
    const d = sortDir === 'asc' ? 1 : -1;
    const k = sortKey;
    arr.sort((a, b) => {
      if (k === 'name') return d * String(a.name).localeCompare(String(b.name));
      const av = k === 'cpu' ? a.cpu ?? 0 : k === 'ram' ? a.rss_mb ?? 0 : a.pid;
      const bv = k === 'cpu' ? b.cpu ?? 0 : k === 'ram' ? b.rss_mb ?? 0 : b.pid;
      return d * (av - bv);
    });
    return arr;
  });

  function apply() {
    setProcRate(rate);
    setProcMax(max);
    sheet = false;
    procs = [];
    live.restart();
  }

  async function killTarget() {
    const p = target;
    target = null;
    if (!p) return;
    try {
      const r = JSON.parse(await kmgr(['kill', String(p.pid)]));
      if (r.ok) toast(`Killed ${r.name || p.name} (${p.pid})`);
      else toast(r.error ? String(r.error) : 'Kill failed');
    } catch {
      toast('Kill failed');
    }
  }
</script>

<div class="proc-head">
  <button class="c-cpu" onclick={() => sortBy('cpu')}>CPU{arrow('cpu')}</button>
  <button class="c-ram" onclick={() => sortBy('ram')}>RAM{arrow('ram')}</button>
  <button class="c-pid" onclick={() => sortBy('pid')}>PID{arrow('pid')}</button>
  <button class="c-name" onclick={() => sortBy('name')}>NAME{arrow('name')}</button>
</div>

<div class="proc-body">
  {#each sorted as p (p.pid)}
    <button class="proc-row" onclick={() => (target = p)} title="Tap to kill">
      <span class="c-cpu">{pct(p.cpu == null ? null : Math.round(p.cpu * 10) / 10)}</span>
      <span class="c-ram">{Math.round(p.rss_mb || 0)} MB</span>
      <span class="c-pid">{p.pid}</span>
      <span class="c-name">{p.name}</span>
    </button>
  {/each}
</div>

<button class="live-fab" onclick={() => (sheet = true)}>
  <Icon name="settings" size={20} />
  Settings
</button>

{#if sheet}
  <div class="scrim" role="presentation" onclick={() => (sheet = false)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>Process list</h3>
    <label class="field">
      Sample rate (ms)
      <input type="number" min="500" max="10000" step="100" bind:value={rate} />
    </label>
    <label class="field">
      Max number of processes
      <input type="number" min="5" max="100" step="1" bind:value={max} />
    </label>
    <div class="actions">
      <button onclick={() => (sheet = false)}>Cancel</button>
      <button onclick={apply}>Apply</button>
    </div>
  </div>
{/if}

<Dialog
  open={!!target}
  title="Kill process?"
  body={target
    ? `${target.name} (PID ${target.pid}) — ${Math.round((target.cpu || 0) * 10) / 10}% CPU, ${Math.round(target.rss_mb || 0)} MB RAM. Force-stop it? Unsaved work in this process will be lost.`
    : ''}
  confirmLabel="Kill"
  onconfirm={killTarget}
  oncancel={() => (target = null)}
/>
