// Browser-side mock of the kmgr backend so the WebUI can be developed
// off-device. Shapes match the real JSON exactly.

function num(v, scale, unit) {
  return { value: String(v), display: `${Math.trunc(v / scale)} ${unit}` };
}

const TREE = {
  groups: [
    {
      id: 'cpu',
      title: 'CPU',
      sections: [
        cluster('policy0', 'Little cluster', 1804800, 300000, 'schedutil'),
        cluster('policy4', 'Big cluster', 2419200, 710400, 'schedutil'),
        cluster('policy7', 'Prime cluster', 3187200, 825600, 'schedutil')
      ]
    },
    {
      id: 'gpu',
      title: 'GPU',
      sections: [
        {
          id: 'kgsl',
          scope: 'gpu.kgsl.kgsl-3d0',
          title: 'Adreno',
          entries: [
            freqEntry('max_freq', 'Maximum GPU frequency', '670000000', '670 MHz', 'low'),
            freqEntry('min_freq', 'Minimum GPU frequency', '180000000', '180 MHz', 'low'),
            enumEntry('governor', 'GPU governor', 'msm-adreno-tz', [
              'msm-adreno-tz',
              'simple_ondemand',
              'performance'
            ]),
            boolEntry('throttling', 'Thermal throttling', '1', 'medium')
          ]
        }
      ]
    },
    {
      id: 'memory',
      title: 'Memory',
      sections: [
        {
          id: 'zram',
          scope: 'memory.zram',
          title: 'zRAM',
          entries: [
            boolEntry('state', 'zRAM state', '1', 'medium'),
            {
              id: 'size',
              scope: 'memory.zram',
              key: 'memory.zram.size',
              label: 'zRAM size',
              type: 'int',
              value: '4294967296',
              display: '4096 MiB',
              choices: [],
              labels: {},
              min: 0,
              max: null,
              step: null,
              unit: 'bytes',
              display_unit: 'MiB',
              scale: 1048576,
              help: 'How much RAM the compressed swap area can hold.',
              risk: 'high',
              handler: 'zram',
              bundle: 'zram',
              is_dir: false,
              children: []
            }
          ]
        },
        {
          id: 'vm',
          scope: 'memory.vm',
          title: 'Virtual memory',
          entries: [
            intEntry('swappiness', 'swappiness', '100', 0, 200),
            intEntry('vfs_cache_pressure', 'vfs_cache_pressure', '100', 0, null),
            intEntry('min_free_kbytes', 'min_free_kbytes', '8192', 0, null)
          ]
        }
      ]
    },
    {
      id: 'misc',
      title: 'Miscellaneous',
      sections: [
        {
          id: 'misc',
          scope: 'misc.misc',
          title: '',
          entries: [
            enumEntry('tcp_cc', 'TCP congestion algorithm', 'cubic', ['reno', 'cubic', 'bbr']),
            {
              id: 'thermal_profile',
              scope: 'misc.misc',
              key: 'misc.misc.thermal_profile',
              label: 'Thermal profile',
              type: 'enum_int',
              value: '10',
              display: 'Dynamic',
              choices: [
                { value: '10', label: 'Dynamic' },
                { value: '9', label: 'Game' },
                { value: '0', label: 'Disabled' }
              ],
              labels: {},
              help: 'Kernel thermal profile. Applied once at boot unless you hold it.',
              risk: 'medium',
              is_dir: false,
              children: []
            }
          ]
        }
      ]
    }
  ]
};

function cluster(policy, title, max, min, gov) {
  return {
    id: 'cluster',
    scope: `cpu.cluster.${policy}`,
    title,
    entries: [
      freqEntry('max_freq', 'Maximum CPU frequency', String(max), `${Math.trunc(max / 1000)} MHz`, 'low'),
      freqEntry('min_freq', 'Minimum CPU frequency', String(min), `${Math.trunc(min / 1000)} MHz`, 'low'),
      enumEntry('governor', 'CPU governor', gov, ['performance', 'powersave', 'schedutil']),
      {
        id: 'governor_params',
        scope: `cpu.cluster.${policy}`,
        key: `cpu.cluster.${policy}.governor_params`,
        label: 'Governor parameters',
        type: 'dir',
        value: null,
        display: null,
        choices: [],
        labels: {},
        help: 'Advanced tunables for the selected governor.',
        risk: 'medium',
        is_dir: true,
        children: [
          { label: 'up_rate_limit_us', path: `${policy}/schedutil/up_rate_limit_us`, value: '1000', type: 'int', writable: true },
          { label: 'down_rate_limit_us', path: `${policy}/schedutil/down_rate_limit_us`, value: '5000', type: 'int', writable: true }
        ]
      }
    ]
  };
}

function base(id, label, type, value, display, risk) {
  return {
    id,
    key: id,
    label,
    type,
    value,
    display,
    choices: [],
    labels: {},
    min: null,
    max: null,
    step: null,
    unit: null,
    display_unit: null,
    scale: null,
    help: null,
    risk,
    is_dir: false,
    children: []
  };
}

function freqEntry(id, label, value, display, risk) {
  return { ...base(id, label, 'freq', value, display, risk), display_unit: 'MHz', scale: 1000 };
}
function intEntry(id, label, value, min, max) {
  return { ...base(id, label, 'int', value, value, 'low'), min, max };
}
function boolEntry(id, label, value, risk) {
  return {
    ...base(id, label, 'bool', value, value === '1' ? 'Enabled' : 'Disabled', risk),
    labels: { 0: 'Disabled', 1: 'Enabled' }
  };
}
function enumEntry(id, label, value, choices) {
  return {
    ...base(id, label, 'enum', value, value, 'low'),
    choices: choices.map((c) => ({ value: c, label: c }))
  };
}

const INFO = {
  ok: true,
  kernel: 'Linux version 4.19.113-generic (builder@localhost) #1 SMP PREEMPT',
  gpu: {
    vendor: 'Qualcomm',
    renderer: 'Adreno (TM) 650',
    gl_version: 'OpenGL ES 3.2 V@502.0 (GIT@d3d44dbc, la0b22fa1a, 1633355031) (Date:10/04/21)'
  },
  android: '13',
  sdk: '33',
  model: 'Example',
  device: 'example',
  wireguard: '1.0.20210606',
  slot: '_a'
};

function sample(i) {
  const load = Math.max(2, Math.min(96, 30 + 25 * Math.sin(i / 4)));
  const gpuBusy = Math.max(1, Math.min(99, 20 + 30 * Math.sin(i / 3 + 1)));
  const cores = [0, 1, 2, 3, 4, 5, 6, 7].map((c) => ({
    cpu: c,
    cur: c < 4 ? 1804800 : c < 6 ? 2419200 : 3187200,
    min: c < 4 ? 300000 : c < 6 ? 710400 : 825600,
    load: Math.round(Math.max(0, Math.min(100, load + (c - 3.5) * 4)) * 10) / 10
  }));
  return {
    t: Date.now(),
    load: Math.round(load * 10) / 10,
    cores,
    clusters: [
      { title: 'Little cluster', governor: 'schedutil', cur: 1804800, max: 1804800 },
      { title: 'Big cluster', governor: 'schedutil', cur: 2419200, max: 2419200 },
      { title: 'Prime cluster', governor: 'schedutil', cur: 3187200, max: 3187200 }
    ],
    cpu_temp: Math.round((38 + 6 * Math.sin(i / 5)) * 10) / 10,
    gpu: { busy: Math.round(gpuBusy * 10) / 10, cur: 305000000, max: 670000000 },
    mem: { total: 5694, free: 1200 + i * 2, available: 3100, cached: 1400 },
    zram: { total: 4096, used: 512, compr: 256, pct: 12.5 },
    battery: {
      capacity: 53,
      capacity_mah: 4530,
      status: 'Charging',
      temp: 33.7,
      current: -313000,
      voltage: 3905000,
      charge_counter: 2401790,
      charge_full: 4996000,
      charge_full_design: 5000000,
      cycles: 3
    },
    loadavg: [1.78, 1.92, 1.75],
    loadavg_full: '1.78 1.92 1.75 8/6555 24193',
    entropy: { avail: 3361, poolsize: 4096 },
    uptime: 123456 + i,
    suspend_count: 12,
    deep_sleep: 41.3,
    deep_sleep_ms: 49470000
  };
}

export async function mockExec(cmd) {
  const c = cmd.trim();
  if (c.startsWith('tree')) return JSON.stringify(TREE);
  if (c.startsWith('info')) return JSON.stringify(INFO);
  if (c.startsWith('get ')) {
    const key = c.slice(4).trim();
    return JSON.stringify({ key, value: 'schedutil', persisted: false });
  }
  if (c.startsWith('set ')) {
    const parts = c.split(/\s+/);
    return JSON.stringify({ ok: true, requested: parts[2], actual: parts[2], key: parts[1] });
  }
  if (c.startsWith('reset')) return JSON.stringify({ ok: true, scope: 'all', restored: [] });
  if (c.startsWith('unpersist')) return JSON.stringify({ ok: true });
  if (c.startsWith('kill '))
    return JSON.stringify({ ok: true, pid: Number(c.split(/\s+/)[1]), signal: 9 });
  if (c.startsWith('settings set')) return JSON.stringify({ ok: true });
  if (c.startsWith('settings')) return JSON.stringify({ ok: true, settings: {} });
  if (c.startsWith('live list'))
    return JSON.stringify({
      ok: true,
      tabs: [
        { id: 'processes', label: 'PROCESSES' },
        { id: 'cpu', label: 'CPU' },
        { id: 'cpustats', label: 'CPU STATS' },
        { id: 'gpu', label: 'GPU' },
        { id: 'ram', label: 'RAM' },
        { id: 'zram', label: 'ZRAM' },
        { id: 'ddr', label: 'DDR BUS' },
        { id: 'io', label: 'I/O' },
        { id: 'wakelocks', label: 'WAKELOCKS' },
        { id: 'thermal', label: 'THERMAL ZONES' },
        { id: 'kernellog', label: 'KERNEL LOG' }
      ]
    });
  if (c.startsWith('dmesg'))
    return JSON.stringify({ ok: true, path: '/sdcard/Download/kernel_log_mock.txt' });
  return JSON.stringify({ ok: false, error: `mock: unknown command '${c}'` });
}

function mockLive(tab, i) {
  const load = Math.max(2, Math.min(96, 30 + 25 * Math.sin(i / 4)));
  switch (tab) {
    case 'ram':
      return { t: i, total_kb: 5831000, free_kb: 3000000 + i, used_kb: 2831000, usage: Math.round(48 + 4 * Math.sin(i / 5)) };
    case 'zram':
      return { t: i, total_b: 2147483648, used_b: 1073741824 + i * 1000, usage: Math.round(50 + 3 * Math.sin(i / 5)) };
    case 'gpu':
      return { t: i, usage: Math.round(20 + 30 * Math.sin(i / 3)), cur_hz: 305000000, min_hz: 180000000, max_hz: 670000000 };
    case 'ddr':
      return { t: i, usage: Math.round(10 + 20 * Math.sin(i / 4)), cur_hz: 400000000, min_hz: 150000000, max_hz: 670000000 };
    case 'io':
      return { t: i, usage: Math.round(5 + 10 * Math.abs(Math.sin(i / 4))), read_bps: 1048576 * (1 + (i % 3)), write_bps: 524288 * (1 + (i % 2)) };
    case 'cpu':
      return {
        t: i,
        cores: [0, 1, 2, 3, 4, 5, 6, 7].map((c) => ({
          i: c,
          on: true,
          load: (c === 7 && i < 2) ? null : Math.round(Math.max(0, Math.min(100, load + (c - 3.5) * 4)) * 10) / 10,
          khz: c < 4 ? 1804800 : c < 6 ? 2419200 : 3187200,
          cluster: c < 4 ? 'Little' : c < 6 ? 'Big' : 'Prime'
        }))
      };
    case 'cpustats':
      return { t: i, rows: [{ khz: 300000, ms: 1000 + i * 10 }, { khz: 1017600, ms: 500 }, { khz: 1804800, ms: 200 + i * 5 }], total_ms: 1700 + i * 15 };
    case 'processes':
      return {
        t: i,
        procs: [
          { pid: 9320, name: 'system_server', cpu: 3.6, rss_mb: 184 },
          { pid: 4122, name: 'surfaceflinger', cpu: 2.1, rss_mb: 120 },
          { pid: 8123, name: 'com.example.app', cpu: 1.4, rss_mb: 210 }
        ]
      };
    case 'wakelocks':
      return { t: i, sources: [{ name: 'smp2p-sleepstate', ms: 23000 }, { name: 'wlan', ms: 8000 }, { name: 'bluetooth_timer', ms: 2000 }] };
    case 'thermal':
      return { t: i, zones: [{ i: 0, type: 'aoss0-usr', c: 38.5 + Math.sin(i) }, { i: 1, type: 'cpu0-usr', c: 41.2 }, { i: 2, type: 'gpuss-usr', c: 36.0 }] };
    default:
      return { t: i };
  }
}

export function mockSpawn(cmd, onLine) {
  if (cmd.includes('dmesg')) {
    let i = 0;
    const timer = setInterval(() => {
      i += 1;
      onLine(JSON.stringify({ n: i, t: `[  ${(i * 0.5).toFixed(6)}] mock kernel message ${i}` }));
    }, 500);
    return () => clearInterval(timer);
  }
  const m = cmd.match(/live (\w+)/);
  if (m) {
    let i = 0;
    const tab = m[1];
    const timer = setInterval(() => {
      i += 1;
      onLine(JSON.stringify(mockLive(tab, i)));
    }, 1000);
    return () => clearInterval(timer);
  }
  if (!cmd.includes('monitor')) return () => {};
  let i = 0;
  const timer = setInterval(() => {
    i += 1;
    onLine(JSON.stringify(sample(i)));
  }, 1000);
  return () => clearInterval(timer);
}
