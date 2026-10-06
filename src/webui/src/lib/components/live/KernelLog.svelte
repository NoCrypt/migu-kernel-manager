<script>
  import Icon from '../Icon.svelte';
  import { klog, matchIndices } from '../../klog.svelte.js';
  import { prefs } from '../../ui.svelte.js';

  let viewport;
  let atBottom = true;

  let base = $derived(klog.lines.filter((l) => l.n > klog.baseline));
  let matches = $derived(matchIndices());

  function onScroll() {
    if (!viewport) return;
    atBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight < 40;
  }

  function toTop() {
    if (viewport) viewport.scrollTop = 0;
  }

  function toBottom() {
    if (viewport) viewport.scrollTop = viewport.scrollHeight;
  }

  // Follow the newest line only while not paused and already at the bottom.
  $effect(() => {
    klog.lines.length;
    if (!klog.paused && atBottom && viewport) viewport.scrollTop = viewport.scrollHeight;
  });

  // Keep the current search match in view.
  $effect(() => {
    klog.term;
    klog.cur;
    if (klog.term && matches.length && viewport) {
      const idx = matches[Math.min(klog.cur, matches.length - 1)];
      const el = viewport.querySelector(`[data-i="${idx}"]`);
      viewport.querySelectorAll('.is-active-match').forEach(activeRow => {
        activeRow.classList.remove('is-active-match');
      });
      if (el) {
        el.scrollIntoView({ block: 'center' });
        el.querySelector('mark').classList.add('is-active-match');
      }
    }
  });

  function esc(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
  function mark(text) {
    const safe = esc(text);
    if (!klog.term) return safe;
    const q = klog.term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    return safe.replace(new RegExp(q, 'gi'), (m) => `<mark>${m}</mark>`);
  }
</script>

<div class="klog">
  <div class="klog-view" bind:this={viewport} onscroll={onScroll}>
    {#each base as l, i (l.n)}
      <div class="klog-entry" data-i={i}>
        {#if prefs.logLines}
          <span class="klog-badge">{l.n}</span>
        {/if}
        <span class="klog-text">{@html mark(l.t)}</span>
      </div>
    {/each}
  </div>
  {#if !klog.searchOpen}
    <div class="klog-jump">
      <button aria-label="Scroll to top" onclick={toTop}>
        <Icon name="chevronUp" size={20} />
      </button>
      <button aria-label="Scroll to bottom" onclick={toBottom}>
        <Icon name="chevronDown" size={20} />
      </button>
    </div>
  {/if}
</div>
