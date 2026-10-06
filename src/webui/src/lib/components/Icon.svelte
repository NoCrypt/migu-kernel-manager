<script>
  let { name, size = 24, stroke = 2 } = $props();
  const P = {
    menu: ['M4 6h16', 'M4 12h16', 'M4 18h16'],
    dashboard: ['M3 3h7v9H3z', 'M14 3h7v5h-7z', 'M14 12h7v9h-7z', 'M3 16h7v5H3z'],
    backups: ['M4 4h16v6H4z', 'M4 14h16v6H4z', 'M9 7h6', 'M9 17h6'],
    cpu: [
      'M9 2v2',
      'M15 2v2',
      'M9 20v2',
      'M15 20v2',
      'M2 9h2',
      'M2 15h2',
      'M20 9h2',
      'M20 15h2',
      'M6 6h12v12H6z'
    ],
    sliders: [
      'M4 6h10',
      'M18 6h2',
      'M4 12h4',
      'M12 12h8',
      'M4 18h10',
      'M18 18h2',
      'M14 4v4',
      'M8 10v4',
      'M14 16v4'
    ],
    display: ['M4 4h16v12H4z', 'M8 20h8', 'M12 16v4'],
    terminal: ['M4 5h16v14H4z', 'M8 9l3 3-3 3', 'M13 15h4'],
    battery: ['M3 8h14v8H3z', 'M17 11h3v2h-3z', 'M5 10v4'],
    props: ['M6 3h9l4 4v14H6z', 'M14 3v5h5', 'M9 13h6', 'M9 17h6'],
    settings: [
      'M12 9a3 3 0 100 6 3 3 0 000-6z',
      'M19 12a7 7 0 00-.1-1l2-1.6-2-3.4-2.4 1a7 7 0 00-1.7-1L14.4 3h-4l-.4 2.6a7 7 0 00-1.7 1l-2.4-1-2 3.4L5.9 11a7 7 0 000 2l-2 1.6 2 3.4 2.4-1a7 7 0 001.7 1l.4 2.6h4l.4-2.6a7 7 0 001.7-1l2.4 1 2-3.4-2-1.6c.1-.3.1-.7.1-1z'
    ],
    info: ['M12 2a10 10 0 100 20 10 10 0 000-20z', 'M12 10v6', 'M12 7h.01'],
    close: ['M6 6l12 12', 'M18 6L6 18'],
    chevron: ['M9 6l6 6-6 6'],
    chevronUp: ['M6 15l6-6 6 6'],
    chevronDown: ['M6 9l6 6 6-6'],
    back: ['M15 6l-6 6 6 6'],
    play: ['M6 4l14 8-14 8z'],
    pause: ['M8 4v16', 'M16 4v16'],
    monitor: ['M2 4h20v14H2z', 'M8 21h8', 'M12 18v3', 'M7 9l3 3-3 3', 'M13 15h4'],
    zap: ['M13 2L3 14h7l-1 8 10-12h-7z'],
    thermo: ['M12 3a2 2 0 00-2 2v8a4 4 0 104 0V5a2 2 0 00-2-2z'],
    plus: ['M12 5v14', 'M5 12h14'],
    search: ['M11 3a8 8 0 100 16 8 8 0 000-16z', 'M21 21l-4.3-4.3'],
    refresh: [
      'M3 12a9 9 0 019-9 9 9 0 016.7 3L21 8',
      'M21 3v5h-5',
      'M21 12a9 9 0 01-9 9 9 9 0 01-6.7-3L3 16',
      'M3 21v-5h5'
    ]
  };
  let d = $derived(P[name] || []);
</script>

<svg
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each d as p}<path d={p} />{/each}
</svg>
