<script>
  import { onMount } from 'svelte';
  import SettingRow from '../lib/components/SettingRow.svelte';
  import PickerSheet from '../lib/components/PickerSheet.svelte';
  import Dialog from '../lib/components/Dialog.svelte';
  import { kmgr } from '../lib/backend.js';
  import { prefs, toast } from '../lib/ui.svelte.js';

  const tabs = [
    { id: 'cpu', label: 'CPU' },
    { id: 'gpu', label: 'GPU' }
  ];

  let tab = $state('cpu');
  let groups = $state([]);
  let loading = $state(true);
  let picker = $state(null);
  let confirm = $state(null);

  async function load() {
    try {
      const t = JSON.parse(await kmgr('tree'));
      groups = t.groups || [];
    } catch {
      groups = [];
    }
    loading = false;
  }

  function sectionsFor(id) {
    return groups.find((g) => g.id === id)?.sections || [];
  }

  onMount(load);

  function pick(entry) {
    if (entry.type === 'custom') {
      toast('Not available yet');
      return;
    }
    picker = { ...entry, isChild: entry.type === 'custom' };
  }

  function pickChild(child) {
    picker = { ...child, key: child.path, help: null, risk: 'low', choices: [], isChild: true };
  }

  function applyValue(entry, value) {
    const run = async () => {
      const r =
        entry.isChild
          ? JSON.parse(await kmgr(`set-path '${entry.path || entry.key}' '${value}'`))
          : JSON.parse(await kmgr(`set ${entry.key} ${value}${entry.persisted ? ' --persist' : ''}`));
      if (r.ok) {
        const actual = r.actual && typeof r.actual === 'object' ? null : r.actual;
        if (actual && String(actual) !== String(value)) toast(`Clamped to ${actual}`);
      } else {
        toast(r.error ? String(r.error) : 'Value rejected');
      }
      picker = null;
      await load();
    };
    if (entry.risk === 'high') {
      confirm = {
        title: entry.label,
        body: entry.help || 'This can affect stability or performance. Apply anyway?',
        run
      };
      picker = null;
    } else {
      run();
    }
  }

  function selectChoice(value) {
    applyValue(picker, value);
  }

  async function persist(entry, on) {
    await kmgr(`${on ? 'persist' : 'unpersist'} ${entry.key}`);
    await load();
  }

  async function persistPath(path, on) {
    await kmgr(`${on ? 'persist-path' : 'unpersist-path'} '${path}'`);
    await load();
  }
</script>

<div class="tabs">
  {#each tabs as t (t.id)}
    <button class="tab" class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
  {/each}
</div>

<div class="page">
  {#if loading}
    <p class="empty">Loading…</p>
  {:else}
    {#each sectionsFor(tab) as sec (sec.scope)}
      {#if sec.title}
        <h3 class="sec">{sec.title}</h3>
      {/if}
      {#each sec.entries as entry (entry.key)}
        {#if entry.children && entry.children.length}
          <h3 class="sec">{entry.label}</h3>
          {#each entry.children as child (child.path)}
            <div class="row">
              <button class="txt" onclick={() => pickChild(child)}>
                <div class="t">{child.label}</div>
                <div class="s">{child.value ?? '—'}</div>
              </button>
              <label class="sw">
                <input
                  type="checkbox"
                  checked={child.persisted}
                  onchange={(e) => persistPath(child.path, e.currentTarget.checked)}
                />
                <i></i>
              </label>
            </div>
          {/each}
        {:else}
          <SettingRow {entry} onpick={pick} onpersist={persist} showHelp={prefs.showHelp} />
        {/if}
      {/each}
    {/each}
    {#if sectionsFor(tab).length === 0}
      <p class="empty">Nothing available on this kernel.</p>
    {/if}
  {/if}
</div>

<PickerSheet entry={picker} onclose={() => (picker = null)} onselect={selectChoice} />

<Dialog
  open={!!confirm}
  title={confirm?.title || ''}
  body={confirm?.body || ''}
  confirmLabel="Apply"
  onconfirm={() => {
    const c = confirm;
    confirm = null;
    c?.run();
  }}
  oncancel={() => (confirm = null)}
/>
