import { exec, spawn, hasKsu } from './ksu.js';

export const KMGR = '/data/adb/modules/kmgr/bin/kmgr';

export function kmgr(args) {
  return exec(`${KMGR} ${args}`);
}

let counter = 0;

// The child self-terminates after this many seconds. The UI transparently
// restarts it while visible, so a leaked monitor (app crash/force-stop, where
// the JS stop() never runs) can never outlive this window.
const MONITOR_LIFETIME = 30;

// Starts `kmgr monitor`, parses NDJSON and calls onSample per tick.
// Returns a stop() that also terminates the native child (the JS API has no
// kill, so we tag the command and pkill it).
export function startMonitor(interval, onSample) {
  let stopped = false;
  let tag = '';
  let stopChild = null;

  function launch() {
    if (stopped) return;
    tag = `kmgr-mon-${(counter++).toString(36)}-${Date.now().toString(36)}`;
    stopChild = spawn(
      `${KMGR} monitor --interval ${interval} --seconds ${MONITOR_LIFETIME} ${tag}`,
      (line) => {
        try {
          onSample(JSON.parse(line));
        } catch {
          /* ignore malformed line */
        }
      },
      () => {
        if (!stopped) setTimeout(launch, 250);
      }
    );
  }
  launch();

  return () => {
    if (stopped) return;
    stopped = true;
    const t = tag;
    try {
      stopChild && stopChild();
    } catch {
      /* ignore */
    }
    if (hasKsu()) exec(`pkill -f ${t}`).catch(() => {});
  };
}
