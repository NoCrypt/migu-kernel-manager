<script>
  import { onMount } from 'svelte';
  import Card from '../lib/components/Card.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import miku from '../assets/miku.webp';
  import { ui, prefs, navigate } from '../lib/ui.svelte.js';
  import { kmgr, startMonitor } from '../lib/backend.js';
  import {
    mhz,
    hzToMhz,
    mib,
    ma,
    pct,
    uptime,
    watts,
    healthLabel,
    chargeStatus,
    isCharging,
    tempC,
    durationMs,
    ratioPct,
    ratioPct2
  } from '../lib/format.js';

  let sample = $state(null);
  let info = $state(null);
  let running = false;
  let stopFn = null;

  let govLine = $derived(
    sample?.clusters ? [...new Set(sample.clusters.map((c) => c.governor).filter(Boolean))].join(', ') : ''
  );
  let memUsed = $derived(sample?.mem ? (sample.mem.total ?? 0) - (sample.mem.available ?? 0) : 0);
  let healthPct = $derived(
    sample?.battery?.charge_full && sample?.battery?.charge_full_design
      ? Math.round((sample.battery.charge_full / sample.battery.charge_full_design) * 100)
      : null
  );

  function start() {
    if (running) return;
    running = true;
    stopFn = startMonitor(prefs.interval, (s) => (sample = s));
  }
  function stop() {
    running = false;
    if (stopFn) stopFn();
    stopFn = null;
  }
  function onVis() {
    if (document.hidden) stop();
    else start();
  }

  onMount(() => {
    kmgr('info')
      .then((r) => (info = JSON.parse(r)))
      .catch(() => (info = null));
    start();
    document.addEventListener('visibilitychange', onVis);
    return () => {
      stop();
      document.removeEventListener('visibilitychange', onVis);
    };
  });
</script>

<div class="page with-fab">
  {#if ui.bootFailed}
    <div class="banner">Saved settings were skipped after a failed boot.</div>
  {/if}

  {#if !sample}
    <div class="loading-center">
      <img src={miku} alt="Waiting for the first sample" />
    </div>
  {:else}
    <Card label="CPU" icon="cpu">
      <div class="grid4">
        {#each sample.cores as c (c.cpu)}
          <div class="c">
            <div class="cur">{mhz(c.cur)}</div>
          </div>
        {/each}
      </div>
      <div class="line"><b>System load:</b> {pct(sample.load)}</div>
      {#each sample.clusters as c (c.title)}
        <div class="line"><b>{c.title} max freq:</b> {mhz(c.max)}</div>
      {/each}
      {#if govLine}
        <div class="line"><b>Governor:</b> {govLine}</div>
      {/if}
    </Card>

    <Card label="GPU" icon="zap">
      <div class="line"><b>Usage:</b> {pct(sample.gpu.busy)}</div>
      <div class="line"><b>Current freq:</b> {hzToMhz(sample.gpu.cur)}</div>
      <div class="line"><b>Max freq:</b> {hzToMhz(sample.gpu.max)}</div>
      {#if info?.gpu}
        <div class="line"><b>Vendor:</b> {info.gpu.vendor || '—'}</div>
        <div class="line"><b>Renderer:</b> {info.gpu.renderer || '—'}</div>
        <div class="line"><b>Version:</b> {info.gpu.gl_version || '—'}</div>
      {/if}
    </Card>

    <Card label="Memory" icon="sliders">
      {#if sample.mem}
        <div class="line"><b>RAM</b></div>
        <div class="line"><b>Total:</b> {mib(sample.mem.total)}</div>
        <div class="line">
          <b>Free:</b>
          {mib(sample.mem.available)} ({ratioPct(sample.mem.available, sample.mem.total)})
        </div>
        <div class="line"><b>Used:</b> {mib(memUsed)} ({ratioPct(memUsed, sample.mem.total)})</div>
      {/if}
      {#if sample.zram}
        <div class="line" style="margin-top:8px"><b>zRAM</b></div>
        <div class="line"><b>Total:</b> {mib(sample.zram.total)}</div>
        <div class="line">
          <b>Used:</b>
          {mib(sample.zram.used)} ({ratioPct(sample.zram.used, sample.zram.total)})
        </div>
      {/if}
    </Card>

    <Card label="Battery" icon="battery">
      <div class="line">
        <b>Capacity:</b>
        {sample.battery.capacity_mah != null ? `${sample.battery.capacity_mah} mAh` : '—'}
      </div>
      <div class="line"><b>Charge level:</b> {pct(sample.battery.capacity)}</div>
      <div class="line">
        <b>Status:</b>
        {chargeStatus(sample.battery.status, sample.battery.current)}
        {#if isCharging(sample.battery.status)}⚡{/if}
        {ma(Math.abs(sample.battery.current ?? 0))}
        {#if watts(sample.battery.voltage, sample.battery.current)}
          · {watts(sample.battery.voltage, sample.battery.current)}
        {/if}
      </div>
      <div class="line">
        <b>Health:</b>
        estimated ~{healthPct}% ({healthLabel(healthPct ?? 0)})
      </div>
      <div class="line"><b>Temperature:</b> {tempC(sample.battery.temp)}</div>
    </Card>

    <Card label="System" icon="info">
      {#if sample.loadavg_full || sample.loadavg}
        <div class="line">
          <b>Loadavg:</b>
          {sample.loadavg_full || (sample.loadavg ?? []).join(' ')}
        </div>
      {/if}
      {#if sample.entropy}
        <div class="line">
          <b>Entropy:</b>
          {sample.entropy.avail}/{sample.entropy.poolsize}
          ({ratioPct2(sample.entropy.avail, sample.entropy.poolsize)} available)
        </div>
      {/if}
      <div class="line"><b>Uptime:</b> {uptime(sample.uptime)}</div>
      {#if sample.deep_sleep_ms != null}
        <div class="line">
          <b>Deep sleep:</b>
          {durationMs(sample.deep_sleep_ms)} ({pct(sample.deep_sleep)})
        </div>
      {:else if sample.deep_sleep != null}
        <div class="line"><b>Deep sleep:</b> {pct(sample.deep_sleep)}</div>
      {/if}
      {#if info?.wireguard}
        <div class="line"><b>Wireguard version:</b> {info.wireguard}</div>
      {/if}
      {#if info?.kernel}
        <div class="line"><b>Kernel:</b> {info.kernel}</div>
      {/if}
    </Card>
  {/if}
</div>

<button class="fab" onclick={() => navigate('live')}>
  <Icon name="monitor" size={20} />
  Live monitor
</button>
