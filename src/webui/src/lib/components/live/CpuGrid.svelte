<script>
  import { onMount } from 'svelte';
  import MiniChart from './MiniChart.svelte';
  import { mountLive } from '../../live.js';
  import { mhz, pct } from '../../format.js';

  let cores = $state([]);
  let hist = $state({});

  const live = mountLive(
    () => 'live cpu --interval 1000',
    (s) => {
      cores = s.cores || [];
      const next = { ...hist };
      for (const c of cores) {
        if (c.load != null) {
          next[c.i] = [...(next[c.i] || []), c.load].slice(-10);
        }
      }
      hist = next;
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });
</script>

<div class="cpu-grid">
  {#each cores as c (c.i)}
    <div class="cpu-tile" class:off={!c.on}>
      <div class="cpu-txt">
        {#if !c.on}
          <div class="l1">CPU {c.i}</div>
          <div class="l3">Offline</div>
        {:else if c.load == null}
          <div class="l2">Sampling</div>
        {:else}
          <div class="l1">CPU {c.i} · {pct(c.load)}</div>
        {/if}
        {#if c.on && c.khz}
          <div class="l2">{mhz(c.khz)}</div>
        {/if}
        <div class="l3">{c.cluster}</div>
      </div>
      {#if c.on}
        <MiniChart values={hist[c.i] || []} />
      {/if}
    </div>
  {/each}
</div>
