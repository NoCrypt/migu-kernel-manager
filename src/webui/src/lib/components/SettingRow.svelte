<script>
  import Icon from './Icon.svelte';

  let { entry, onpick, onpersist, showHelp = true } = $props();

  let hasChildren = $derived(!!(entry.children && entry.children.length));
  let persistable = $derived(
    !['dir', 'auto_glob', 'custom'].includes(entry.type) && !!entry.key
  );
  let subtitle = $derived(
    entry.display ?? entry.value ?? (hasChildren ? 'Open' : '—')
  );
</script>

<div class="row">
  <button class="txt" onclick={() => onpick(entry)}>
    <div class="t">{entry.label}</div>
    <div class="s">{subtitle}</div>
    {#if showHelp && entry.help}
      <div class="help">{entry.help}</div>
    {/if}
  </button>
  {#if persistable}
    <label class="sw">
      <input
        type="checkbox"
        checked={entry.persisted}
        onchange={(e) => onpersist(entry, e.currentTarget.checked)}
      />
      <i></i>
    </label>
  {:else if hasChildren}
    <span class="chev"><Icon name="chevron" size={20} /></span>
  {/if}
</div>
