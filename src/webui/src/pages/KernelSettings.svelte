<script>
  import { onMount } from 'svelte';
  import SettingRow from '../lib/components/SettingRow.svelte';
  import PickerSheet from '../lib/components/PickerSheet.svelte';
  import Dialog from '../lib/components/Dialog.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { kmgr, KMGR } from '../lib/backend.js';
  import { spawn, exec } from '../lib/ksu.js';
  import { prefs, toast } from '../lib/ui.svelte.js';

  const tabs = [
    { id: 'io', label: 'I/O' },
    { id: 'memory', label: 'Memory' },
    { id: 'scheduler', label: 'Scheduler' },
    { id: 'misc', label: 'Miscellaneous' },
    { id: 'custom', label: 'Custom tunables' }
  ];

  let tab = $state('io');
  let groups = $state([]);
  let custom = $state([]);
  let loading = $state(true);
  let picker = $state(null);
  let confirm = $state(null);
  let hold = $state(false);
  let holdOpen = $state(false);
  let addOpen = $state(false);
  let addPath = $state('');

  async function load() {
    try {
      const t = JSON.parse(await kmgr('tree'));
      groups = t.groups || [];
    } catch {
      groups = [];
    }
    try {
      custom = JSON.parse(await kmgr('custom list')).tunables || [];
    } catch {
      custom = [];
    }
    try {
      hold = JSON.parse(await kmgr('thermal-hold')).on === true;
    } catch {
      hold = false;
    }
    loading = false;
  }

  function sectionsFor(id) {
    if (id === 'misc') {
      return (groups.find((g) => g.id === 'misc')?.sections || []).filter((s) => s.id !== 'custom');
    }
    return groups.find((g) => g.id === id)?.sections || [];
  }

  onMount(load);

  function pick(entry) {
    picker = { ...entry, isChild: entry.type === 'custom' };
  }

  function pickChild(child) {
    picker = { ...child, key: child.path, help: null, risk: 'low', choices: [], isChild: true };
  }

  function pickCustom(item) {
    picker = {
      label: item.path,
      value: item.value,
      type: item.type === 'missing' ? 'string' : item.type,
      choices: [],
      help: null,
      risk: 'low',
      key: item.path,
      isChild: true
    };
  }

  function applyValue(entry, value, kind) {
    const run = async () => {
      const r =
        kind === 'path' || entry.isChild
          ? JSON.parse(await kmgr(['set-path', entry.path || entry.key, String(value)]))
          : JSON.parse(
              await kmgr(
                entry.persisted
                  ? ['set', entry.key, String(value), '--persist']
                  : ['set', entry.key, String(value)]
              )
            );
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
    await kmgr([on ? 'persist' : 'unpersist', entry.key]);
    await load();
  }

  async function persistPath(path, on) {
    await kmgr([on ? 'persist-path' : 'unpersist-path', path]);
    await load();
  }

  async function toggleHold(on) {
    hold = on;
    await kmgr(['thermal-hold', on ? 'on' : 'off']);
    if (on) spawn(KMGR, ['hold-thermal', '--interval', '10000'], () => {});
    else exec("pkill -f '[h]old-thermal'").catch(() => {});
  }

  async function addCustom() {
    const r = JSON.parse(await kmgr(['custom', 'add', addPath.trim()]));
    if (r.ok) {
      addOpen = false;
      addPath = '';
      toast('Tunable added');
      await load();
    } else {
      toast(r.error ? String(r.error) : 'Could not add');
    }
  }

  async function removeCustom(path) {
    await kmgr(['custom', 'remove', path]);
    await load();
  }
</script>

<div class="tabs">
  {#each tabs as t (t.id)}
    <button class="tab" class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
  {/each}
</div>

<div class="page" class:with-fab={tab === 'custom'}>
  {#if loading}
    <p class="empty">Loading…</p>
  {:else if tab === 'custom'}
    {#if custom.length === 0}
      <p class="empty">No custom tunables yet. Tap “Add a new tunable”.</p>
    {:else}
      {#each custom as item (item.path)}
        <div class="row">
          <button class="txt" onclick={() => pickCustom(item)}>
            <div class="t">{item.path.split('/').pop()}</div>
            <div class="s">{item.value ?? 'unavailable'}</div>
            <div class="help">{item.path}</div>
          </button>
          <label class="sw">
            <input
              type="checkbox"
              checked={item.persisted}
              onchange={(e) => persistPath(item.path, e.currentTarget.checked)}
            />
            <i></i>
          </label>
          <button class="icon-btn" aria-label="Remove" onclick={() => removeCustom(item.path)}>
            <Icon name="close" size={18} />
          </button>
        </div>
      {/each}
    {/if}
  {:else}
    {#each sectionsFor(tab) as sec (sec.scope)}
      {#if tab === 'misc'}
        <div class="row">
          <button class="txt" onclick={() => (holdOpen = true)}>
            <div class="t">Hold thermal profile</div>
            <div class="s">{hold ? 'Enabled' : 'Disabled'} · may override other tools</div>
            {#if prefs.showHelp}
              <div class="help">Re-applies the saved thermal profile every 10s. Off by default.</div>
            {/if}
          </button>
          <label class="sw">
            <input
              type="checkbox"
              checked={hold}
              onchange={(e) => toggleHold(e.currentTarget.checked)}
            />
            <i></i>
          </label>
        </div>
      {/if}
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

{#if tab === 'custom'}
  <button class="fab" onclick={() => (addOpen = true)}>
    <Icon name="plus" size={20} />
    Add a new tunable
  </button>
{/if}

<PickerSheet entry={picker} onclose={() => (picker = null)} onselect={selectChoice} />

{#if addOpen}
  <div class="scrim" role="presentation" onclick={() => (addOpen = false)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>Add a custom tunable</h3>
    <p class="desc">
      Advanced. Enter an absolute path under /sys, /proc/sys, /dev/cpuset, /dev/stune or
      /dev/cpuctl. Only add nodes you understand.
    </p>
    <div class="numrow">
      <input type="text" bind:value={addPath} placeholder="/sys/..." />
    </div>
    <div class="actions">
      <button onclick={() => (addOpen = false)}>Cancel</button>
      <button onclick={addCustom}>Add</button>
    </div>
  </div>
{/if}

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

{#if holdOpen}
  <div class="scrim" role="presentation" onclick={() => (holdOpen = false)}></div>
  <div class="sheet" role="dialog" aria-modal="true">
    <h3>Hold thermal profile</h3>
    <p class="desc">
      Keeps re-applying the saved thermal profile every 10 seconds while the screen is on. This may
      override other tools that change it. Off by default.
    </p>
    <div class="actions">
      <button onclick={() => (holdOpen = false)}>Cancel</button>
      <button
        onclick={() => {
          toggleHold(!hold);
          holdOpen = false;
        }}
      >
        {hold ? 'Disable' : 'Enable'}
      </button>
    </div>
  </div>
{/if}
