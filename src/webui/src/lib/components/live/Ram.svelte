<script>
  import { onMount } from 'svelte';
  import ChartPage from './ChartPage.svelte';
  import { mountLive } from '../../live.js';
  import { prefs } from '../../ui.svelte.js';
  import { gib, pctSmart } from '../../format.js';

  let hist = $state([]);
  let stats = $state([]);
  let usage = $state('—');

  const live = mountLive(
    () => `live ram --interval ${Math.max(200, Math.round(prefs.chartInterval / 2))}`,
    (s) => {
      if (s.usage != null) hist = [...hist, s.usage].slice(-60);
      usage = s.usage == null ? '—' : `${Math.round(s.usage)}%`;
      stats = [
        { k: 'USAGE', v: usage },
        { k: 'FREE', v: gib(s.free_kb) },
        { k: 'USED', v: gib(s.used_kb) },
        { k: 'TOTAL', v: gib(s.total_kb) }
      ];
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });
</script>

<ChartPage {hist} {stats} label="RAM usage {usage}" />
