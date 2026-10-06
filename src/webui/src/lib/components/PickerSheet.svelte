<script>
  let { entry, onclose, onselect } = $props();

  let kind = $derived(entry?.type);
  let isNumeric = $derived(kind === 'int' || kind === 'freq');
  let num = $state(0);
  let text = $state('');

  // The backend may store a value in a small unit (bytes, kHz) while the
  // registry asks for a friendlier one (MiB, MHz) via display_unit/scale.
  let scale = $derived(entry && Number(entry.scale) > 1 ? Number(entry.scale) : 1);
  let unitLabel = $derived(entry?.display_unit || '');
  let scaled = $derived(isNumeric && scale > 1 && !!unitLabel);
  let minV = $derived(entry?.min != null ? Number(entry.min) / scale : null);
  let maxV = $derived(entry?.max != null ? Number(entry.max) / scale : null);

  let options = $derived.by(() => {
    if (!entry) return [];
    if (entry.choices && entry.choices.length) return entry.choices;
    if (entry.type === 'bool') {
      const labels =
        entry.labels && Object.keys(entry.labels).length
          ? entry.labels
          : { 0: 'Disabled', 1: 'Enabled' };
      return Object.entries(labels)
        .map(([value, label]) => ({ value, label }))
        .sort((a, b) => Number(a.value) - Number(b.value));
    }
    return [];
  });

  $effect(() => {
    if (!entry) return;
    num = Number(entry.value ?? 0) / scale;
    text = String(entry.value ?? '');
  });

  let step = $derived.by(() => {
    const raw = Number(entry?.step);
    return raw > 0 && raw >= scale ? raw / scale : 1;
  });

  function clamp(v) {
    let n = Number(v);
    if (Number.isNaN(n)) n = 0;
    if (minV != null && n < minV) n = minV;
    if (maxV != null && n > maxV) n = maxV;
    return n;
  }

  let rawValue = $derived(Math.round(clamp(num) * scale));
</script>

{#if entry}
  <div class="scrim" role="presentation" onclick={onclose}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>{entry.label}</h3>
    {#if entry.help}
      <p class="desc">{entry.help}</p>
    {/if}

    {#if options.length}
      <div class="options">
        {#each options as c (c.value)}
          <button
            class="opt"
            class:on={String(c.value) === String(entry.value)}
            onclick={() => onselect(String(c.value))}
          >
            {c.label}
          </button>
        {/each}
      </div>
      <div class="actions">
        <button onclick={onclose}>Cancel</button>
      </div>
    {:else if isNumeric}
      <div class="numrow">
        <button onclick={() => (num = clamp(Number(num) - step))}>−</button>
        <input type="number" bind:value={num} min={minV} max={maxV} step={step} />
        {#if scaled}<span class="unit">{unitLabel}</span>{/if}
        <button onclick={() => (num = clamp(Number(num) + step))}>+</button>
      </div>
      {#if scaled}
        <p class="conv">
          {rawValue.toLocaleString()}{entry.unit ? ` ${entry.unit}` : ''}
        </p>
      {/if}
      <div class="actions">
        <button onclick={onclose}>Cancel</button>
        <button onclick={() => onselect(String(rawValue))}>Apply</button>
      </div>
    {:else}
      <div class="numrow">
        <input type="text" bind:value={text} />
      </div>
      <div class="actions">
        <button onclick={onclose}>Cancel</button>
        <button onclick={() => onselect(text)}>Apply</button>
      </div>
    {/if}
  </div>
{/if}
