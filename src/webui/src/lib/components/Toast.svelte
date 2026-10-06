<script>
  import { ui } from '../ui.svelte.js';
</script>

<div class="toast-wrap" aria-live="polite">
  {#each ui.toasts as t (t.id)}
    <div class="toast">{t.message}</div>
  {/each}
</div>
