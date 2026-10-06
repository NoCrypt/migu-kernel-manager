<script>
  import { onMount } from 'svelte';
  import { drawArea } from '../../chartDraw.js';

  let { hist = [], stats = [], label = '' } = $props();
  let el;

  function draw() {
    drawArea(el, hist, { spacing: 20, fit: true, lineWidth: 1 });
  }

  $effect(() => {
    hist;
    stats;
    draw();
  });

  onMount(() => {
    const ro = new ResizeObserver(() => draw());
    ro.observe(el);
    return () => ro.disconnect();
  });
</script>

<div class="chart-page">
  <div class="chart-frame" aria-label={label}>
    <canvas bind:this={el}></canvas>
  </div>
  <div class="chart-stats">
    {#each stats as s (s.k)}
      <div class="chart-stat">
        <div class="k">{s.k}</div>
        <div class="v">{s.v}</div>
      </div>
    {/each}
  </div>
</div>
