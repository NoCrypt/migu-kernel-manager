<script>
  import { onMount } from 'svelte';
  import ChartPage from './ChartPage.svelte';
  import { mountLive } from '../../live.js';
  import { prefs } from '../../ui.svelte.js';
  import { gibFromBytes } from '../../format.js';

  let hist = $state([]);
  let stats = $state([]);
  let usage = $state('—');
  let disabled = $state(false);

  const live = mountLive(
    () => `live zram --interval ${prefs.chartInterval}`,
    (s) => {
      disabled = !s.total_b;
      if (s.usage != null && !disabled) hist = [...hist, s.usage].slice(-60);
      usage = s.usage == null ? '—' : `${Math.round(s.usage)}%`;
      stats = [
        { k: 'USAGE', v: usage },
        { k: 'FREE', v: gibFromBytes((s.total_b || 0) - (s.used_b || 0)) },
        { k: 'USED', v: gibFromBytes(s.used_b) },
        { k: 'TOTAL', v: gibFromBytes(s.total_b) }
      ];
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });
</script>

{#if disabled}
  <p class="empty">zRAM is disabled</p>
{:else}
  <ChartPage {hist} {stats} label="zRAM usage {usage}" />
{/if}
