<script>
  import { onMount } from 'svelte';
  import BottomBar from './lib/components/BottomBar.svelte';
  import Drawer from './lib/components/Drawer.svelte';
  import Toast from './lib/components/Toast.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import LiveMonitor from './pages/LiveMonitor.svelte';
  import KernelSettings from './pages/KernelSettings.svelte';
  import CpuGpu from './pages/CpuGpu.svelte';
  import Display from './pages/Display.svelte';
  import Props from './pages/Props.svelte';
  import Settings from './pages/Settings.svelte';
  import Placeholder from './lib/components/Placeholder.svelte';
  import { ui, routeFromHash } from './lib/ui.svelte.js';
  import { enableEdgeToEdge } from './lib/ksu.js';

  const titles = {
    dashboard: 'Dashboard',
    live: 'Live monitor',
    cpu: 'CPU and GPU',
    settings: 'Kernel settings',
    display: 'Display control',
    props: 'Build.prop editor',
    prefs: 'Settings'
  };
  let title = $derived(titles[ui.route] || 'Migu Kernel Manager');

  onMount(() => {
    enableEdgeToEdge(true);
    ui.route = routeFromHash();
    const onHash = () => (ui.route = routeFromHash());
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  });
</script>

{#if ui.route === 'dashboard'}
  <Dashboard />
{:else if ui.route === 'live'}
  <LiveMonitor />
{:else if ui.route === 'settings'}
  <KernelSettings />
{:else if ui.route === 'cpu'}
  <CpuGpu />
{:else if ui.route === 'display'}
  <Display />
{:else if ui.route === 'props'}
  <Props />
{:else if ui.route === 'prefs'}
  <Settings />
{:else}
  <Placeholder title={title} />
{/if}

{#if ui.route !== 'live'}
  <BottomBar {title} />
{/if}
<Drawer />
<Toast />

<div class="status-scrim" aria-hidden="true"></div>
