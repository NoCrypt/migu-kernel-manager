<script>
  import Icon from './Icon.svelte';
  import { ui } from '../ui.svelte.js';
  let { title = '' } = $props();
</script>

<div class="bottom-bar">
  <span class="title">{title}</span>
  <button class="burger" aria-label="Open menu" onclick={() => (ui.drawer = true)}>
    <Icon name="menu" size={24} />
  </button>
</div>
