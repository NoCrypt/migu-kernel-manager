<script>
  import { onMount } from 'svelte';
  import ChartPage from './ChartPage.svelte';
  import { mountLive } from '../../live.js';
  import { prefs } from '../../ui.svelte.js';
  import { busSpeed } from '../../format.js';

  let hist = $state([]);
  let stats = $state([]);
  let usage = $state('—');

  const live = mountLive(
    () => `live ddr --interval ${prefs.chartInterval}`,
    (s) => {
      if (s.usage != null) hist = [...hist, s.usage].slice(-60);
      usage = s.usage == null ? '—' : `${Math.round(s.usage)}%`;
      stats = [
        { k: 'USAGE', v: usage },
        { k: 'SPEED', v: busSpeed(s.cur_hz) },
        { k: 'MAX SPEED', v: busSpeed(s.max_hz) },
        { k: 'MIN SPEED', v: busSpeed(s.min_hz) }
      ];
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });
</script>

<ChartPage {hist} {stats} label="DDR bus usage {usage}" />
