<script>
  let {
    open = false,
    title = '',
    body = '',
    confirmLabel = 'Confirm',
    onconfirm = () => {},
    oncancel = () => {}
  } = $props();
</script>

{#if open}
  <div class="dialog-scrim" role="presentation" onclick={oncancel}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <h3>{title}</h3>
      <p>{body}</p>
      <div class="actions">
        <button onclick={oncancel}>Cancel</button>
        <button onclick={onconfirm}>{confirmLabel}</button>
      </div>
    </div>
  </div>
{/if}
