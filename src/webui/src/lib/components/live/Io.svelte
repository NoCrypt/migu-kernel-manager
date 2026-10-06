<script>
  import { onMount } from 'svelte';
  import ChartPage from './ChartPage.svelte';
  import { mountLive } from '../../live.js';
  import { prefs } from '../../ui.svelte.js';
  import { bps } from '../../format.js';

  let hist = $state([]);
  let stats = $state([]);
  let usage = $state('—');

  const live = mountLive(
    () => `live io --interval ${prefs.chartInterval}`,
    (s) => {
      if (s.usage != null) hist = [...hist, s.usage].slice(-60);
      usage = s.usage == null ? '—' : `${Math.round(s.usage)}%`;
      stats = [
        { k: 'USAGE', v: usage },
        { k: 'READ SPEED', v: bps(s.read_bps) },
        { k: 'WRITE SPEED', v: bps(s.write_bps) }
      ];
    }
  );

  onMount(() => {
    live.attach();
    return () => live.detach();
  });
</script>

<ChartPage {hist} {stats} label="I/O usage {usage}" />
