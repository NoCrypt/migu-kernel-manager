<script>
  import { onMount } from 'svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { kmgr } from '../lib/backend.js';
  import { toast } from '../lib/ui.svelte.js';

  let query = $state('');
  let props = $state([]);
  let total = $state(0);
  let loading = $state(true);
  let editor = $state(null); // { key, value, isNew }
  let searchTimer = null;

  async function load(q = query) {
    try {
      const r = JSON.parse(await kmgr(`props list ${q}`.trim()));
      props = r.props || [];
      total = r.total || 0;
    } catch {
      props = [];
      total = 0;
    }
    loading = false;
  }

  onMount(() => load(''));

  function onSearch(e) {
    query = e.currentTarget.value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => load(query), 250);
  }

  async function toggle(item, on) {
    const r = JSON.parse(
      await kmgr(on ? `props set '${item.key}' '${item.value}'` : `props unset '${item.key}'`)
    );
    if (r.ok) {
      props = props.map((p) => (p.key === item.key ? { ...p, overridden: on } : p));
      toast(on ? 'Override added · reboot required' : 'Override removed · reboot required');
    } else {
      toast(r.error ? String(r.error) : 'Failed');
    }
  }

  async function saveEditor() {
    const { key, value } = editor;
    if (!key) {
      toast('Enter a key');
      return;
    }
    const r = JSON.parse(await kmgr(`props set '${key}' '${value}'`));
    if (r.ok) {
      editor = null;
      toast('Saved · reboot required');
      await load();
    } else {
      toast(r.error ? String(r.error) : 'Failed');
    }
  }
</script>

<div class="search">
  <Icon name="search" size={20} />
  <input
    type="search"
    placeholder="Search build properties"
    value={query}
    oninput={onSearch}
  />
</div>

<div class="page with-fab">
  {#if loading}
    <p class="empty">Loading…</p>
  {:else}
    <p class="info-note">
      <Icon name="info" size={20} />
      Changes are written to the module's system.prop and take effect after a reboot. Real partitions are never touched.
    </p>
    {#each props as item (item.key)}
      <div class="row">
        <button class="txt" onclick={() => (editor = { key: item.key, value: item.value, isNew: false })}>
          <div class="t prop-key">{item.key}</div>
          <div class="s">{item.value === '' ? '(empty)' : item.value}</div>
        </button>
        <label class="sw">
          <input
            type="checkbox"
            checked={item.overridden}
            onchange={(e) => toggle(item, e.currentTarget.checked)}
          />
          <i></i>
        </label>
      </div>
    {/each}
    {#if props.length === 0}
      <p class="empty">No matching properties.</p>
    {:else if total > props.length}
      <p class="empty">Showing {props.length} of {total}. Search to narrow down.</p>
    {/if}
  {/if}
</div>

<button class="fab" onclick={() => (editor = { key: '', value: '', isNew: true })}>
  <Icon name="plus" size={20} />
  Add entry
</button>

{#if editor}
  <div class="scrim" role="presentation" onclick={() => (editor = null)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>{editor.isNew ? 'Add property' : 'Edit property'}</h3>
    <p class="desc">Written to system.prop. Reboot required.</p>
    <div class="field">
      <label for="pk">Key</label>
      <input id="pk" type="text" bind:value={editor.key} placeholder="ro.example.setting" />
    </div>
    <div class="field">
      <label for="pv">Value</label>
      <input id="pv" type="text" bind:value={editor.value} placeholder="1" />
    </div>
    <div class="actions">
      <button onclick={() => (editor = null)}>Cancel</button>
      <button onclick={saveEditor}>Save</button>
    </div>
  </div>
{/if}
