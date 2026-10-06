<script>
  import { onMount } from 'svelte';
  import { drawArea } from '../../chartDraw.js';

  let { values = [], slots = 10, lineWidth = 2 } = $props();
  let el;

  function draw() {
    drawArea(el, values, { slots, lineWidth });
  }

  $effect(() => {
    values;
    draw();
  });

  onMount(() => {
    const ro = new ResizeObserver(() => draw());
    ro.observe(el);
    return () => ro.disconnect();
  });
</script>

<canvas bind:this={el}></canvas>
