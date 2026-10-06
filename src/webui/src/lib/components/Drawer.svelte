<script>
  import Icon from './Icon.svelte';
  import { ui, navigate } from '../ui.svelte.js';

  const items = [
    { route: 'dashboard', label: 'Dashboard', icon: 'dashboard' },
    { route: 'cpu', label: 'CPU and GPU', icon: 'cpu' },
    { route: 'settings', label: 'Kernel settings', icon: 'sliders' },
    { route: 'display', label: 'Display control', icon: 'display' },
    { route: 'props', label: 'Build.prop editor', icon: 'props' }
  ];
</script>

{#if ui.drawer}
  <div class="scrim" role="presentation" onclick={() => (ui.drawer = false)}></div>
{/if}
<nav class="drawer" class:open={ui.drawer} aria-hidden={!ui.drawer}>
  <div class="top">
    <button aria-label="Settings" onclick={() => navigate('prefs')}>
      <Icon name="settings" size={24} />
    </button>
    <button aria-label="Close" onclick={() => (ui.drawer = false)}>
      <Icon name="close" size={24} />
    </button>
  </div>
  {#each items as it (it.route)}
    <button class="item" class:on={ui.route === it.route} onclick={() => navigate(it.route)}>
      <Icon name={it.icon} size={24} />
      <span>{it.label}</span>
    </button>
  {/each}
</nav>
