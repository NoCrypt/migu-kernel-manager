<script>
  import { onMount } from 'svelte';
  import { mountLive } from '../../live.js';
  import { shortDur, celsius } from '../../format.js';

  let { kind = 'wakelocks' } = $props();

  const isWake = $derived(kind === 'wakelocks');
  const args = $derived(isWake ? 'live wakelocks --interval 2000' : 'live thermal --interval 2000');

  let pm = $state([]);
  let items = $state([]);
  let sel = $state(null);

  const live = mountLive(
    () => args,
    (s) => {
      if (isWake) {
        pm = s.pm || [];
        items = s.sources || [];
      } else {
        items = s.zones || [];
      }
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });
</script>

{#if isWake}
  <h3 class="sec">Power manager</h3>
  <div class="simple-list">
    {#each pm as it, i (`pm-${i}`)}
      <button class="item" onclick={() => (sel = { kind: 'pm', item: it })}>
        <div class="t">{it.name}</div>
        <div class="s">{shortDur(it.ms)}</div>
      </button>
    {/each}
    {#if pm.length === 0}
      <p class="empty">No PowerManager wake locks.</p>
    {/if}
  </div>

  <h3 class="sec">Kernel wakeups</h3>
  <div class="simple-list">
    {#each items as it, i (`k-${i}`)}
      <button class="item" onclick={() => (sel = { kind: 'k', item: it })}>
        <div class="t">{it.name}</div>
        <div class="s">{shortDur(it.ms)}</div>
      </button>
    {/each}
    {#if items.length === 0}
      <p class="empty">No kernel wakeup activity.</p>
    {/if}
  </div>
{:else}
  <h3 class="sec">Thermal zones</h3>
  <div class="simple-list">
    {#each items as it (it.i)}
      <div class="item">
        <div class="t">thermal_zone{it.i} - {it.type}</div>
        <div class="s">{celsius(it.c)}</div>
      </div>
    {/each}
    {#if items.length === 0}
      <p class="empty">No thermal zones.</p>
    {/if}
  </div>
{/if}

{#if sel}
  <div class="scrim" role="presentation" onclick={() => (sel = null)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>{sel.item.name}</h3>
    <p class="desc">{sel.kind === 'pm' ? 'PowerManager wake lock' : 'Kernel wakeup source'}</p>
    <div class="kv"><span>Times the wakelock was activated</span><span>{sel.item.active ?? '—'}</span></div>
    <div class="kv"><span>Times the wakelock aborted suspend</span><span>{sel.item.wakeup ?? '—'}</span></div>
    <div class="kv"><span>Total time it has been active</span><span>{shortDur(sel.item.ms)}</span></div>
    {#if sel.kind === 'k'}
      <div class="kv"><span>Events</span><span>{sel.item.events ?? '—'}</span></div>
      <div class="kv"><span>Expired</span><span>{sel.item.expire ?? '—'}</span></div>
      <div class="kv"><span>Longest active</span><span>{shortDur(sel.item.max_ms)}</span></div>
    {/if}
    <div class="actions">
      <button onclick={() => (sel = null)}>Close</button>
    </div>
  </div>
{/if}
