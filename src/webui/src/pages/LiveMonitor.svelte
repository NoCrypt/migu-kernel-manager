<script>
  import { onMount } from 'svelte';
  import Icon from '../lib/components/Icon.svelte';
  import Gpu from '../lib/components/live/Gpu.svelte';
  import Ram from '../lib/components/live/Ram.svelte';
  import Zram from '../lib/components/live/Zram.svelte';
  import Ddr from '../lib/components/live/Ddr.svelte';
  import Io from '../lib/components/live/Io.svelte';
  import CpuGrid from '../lib/components/live/CpuGrid.svelte';
  import CpuStats from '../lib/components/live/CpuStats.svelte';
  import ProcessTable from '../lib/components/live/ProcessTable.svelte';
  import SimpleList from '../lib/components/live/SimpleList.svelte';
  import KernelLog from '../lib/components/live/KernelLog.svelte';
  import { kmgr } from '../lib/backend.js';
  import { prefs, setLiveTab, navigate } from '../lib/ui.svelte.js';
  import {
    klog,
    attach as klogAttach,
    detach as klogDetach,
    togglePause,
    openSearch,
    closeSearch,
    setTerm,
    nextMatch,
    clearView,
    toggleLineNumbers,
    exportLog,
    matchIndices
  } from '../lib/klog.svelte.js';

  const NO_SCROLL = new Set(['gpu', 'ram', 'zram', 'ddr', 'io', 'cpustats', 'kernellog']);

  let tabs = $state([]);
  let tab = $state(prefs.liveTab);
  let strip;

  let matchCount = $derived(matchIndices().length);

  onMount(async () => {
    try {
      const r = JSON.parse(await kmgr('live list'));
      tabs = r.tabs || [];
    } catch {
      tabs = [];
    }
    if (!tabs.some((t) => t.id === tab)) tab = tabs[0]?.id || '';
    center();
  });

  // Own the kernel-log child only while its tab is visible.
  $effect(() => {
    if (tab === 'kernellog') {
      klogAttach();
      return () => klogDetach();
    }
  });

  function pick(id) {
    tab = id;
    setLiveTab(id);
  }

  function back() {
    if (tab === 'kernellog' && klog.searchOpen) {
      closeSearch();
      return;
    }
    navigate('dashboard');
  }

  function center() {
    if (!strip) return;
    const el = strip.querySelector('.live-tab.on');
    if (!el) return;
    const left = el.offsetLeft + el.offsetWidth / 2 - strip.clientWidth / 2;
    strip.scrollTo({ left, behavior: 'smooth' });
  }

  $effect(() => {
    tab;
    tabs;
    center();
  });
</script>

<div class="live-shell">
  <div class="live-tabs" bind:this={strip}>
    {#each tabs as t (t.id)}
      <button class="live-tab" class:on={tab === t.id} onclick={() => pick(t.id)}>{t.label}</button>
    {/each}
  </div>

  <div class="live-content" class:noscroll={NO_SCROLL.has(tab)}>
    {#if tab === 'processes'}
      <ProcessTable />
    {:else if tab === 'cpu'}
      <CpuGrid />
    {:else if tab === 'cpustats'}
      <CpuStats />
    {:else if tab === 'gpu'}
      <Gpu />
    {:else if tab === 'ram'}
      <Ram />
    {:else if tab === 'zram'}
      <Zram />
    {:else if tab === 'ddr'}
      <Ddr />
    {:else if tab === 'io'}
      <Io />
    {:else if tab === 'wakelocks'}
      <SimpleList kind="wakelocks" />
    {:else if tab === 'thermal'}
      <SimpleList kind="thermal" />
    {:else if tab === 'kernellog'}
      <KernelLog />
    {:else if tabs.length === 0}
      <p class="empty">No live data sources on this kernel.</p>
    {/if}
  </div>

  <div class="live-bar">
    <button class="back" aria-label="Back" onclick={back}>
      <Icon name="back" size={24} />
    </button>
    {#if tab === 'kernellog'}
      <div class="bar-actions">
        {#if klog.searchOpen}
          <div class="search bar-search">
            <Icon name="search" size={18} />
            <input
              type="text"
              placeholder="Search kernel log"
              value={klog.term}
              oninput={(e) => setTerm(e.currentTarget.value)}
            />
          </div>
          <span class="muted">{matchCount ? `${Math.min(klog.cur + 1, matchCount)} of ${matchCount}` : '0 of 0'}</span>
          <button aria-label="Previous match" onclick={() => nextMatch(-1)}>
            <Icon name="back" size={22} />
          </button>
          <button aria-label="Next match" onclick={() => nextMatch(1)}>
            <Icon name="chevron" size={22} />
          </button>
        {:else}
          <button aria-label="Search" onclick={openSearch}>
            <Icon name="search" size={22} />
          </button>
          <button aria-label={klog.paused ? 'Resume' : 'Pause'} onclick={togglePause}>
            <Icon name={klog.paused ? 'play' : 'pause'} size={22} />
          </button>
          <button aria-label="Menu" onclick={() => (klog.menu = true)}>
            <Icon name="menu" size={22} />
          </button>
        {/if}
      </div>
    {:else}
      <span></span>
    {/if}
  </div>
</div>

{#if tab === 'kernellog' && klog.menu}
  <div class="scrim" role="presentation" onclick={() => (klog.menu = false)}></div>
  <div class="menu" role="menu">
    <button class="opt" onclick={toggleLineNumbers}>
      {prefs.logLines ? 'Disable line number' : 'Enable line number'}
    </button>
    <button class="opt" onclick={exportLog}>Export</button>
    <button class="opt" onclick={clearView}>Clear</button>
  </div>
{/if}
