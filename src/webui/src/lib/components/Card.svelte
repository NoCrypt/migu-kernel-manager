<script>
  import Icon from './Icon.svelte';
  let { label, icon = '', children } = $props();
</script>

<section class="card">
  <span class="chip">
    {#if icon}<Icon name={icon} size={20} />{/if}
    {label}
  </span>
  {@render children()}
</section>
