import { exec, execArgs, spawn, hasKsu } from './ksu.js';

export const KMGR = '/data/adb/modules/kmgr/bin/kmgr';

// Must match SCHEMA_VERSION in src/kmgr/src/main.rs. A mismatch means the module
// binary and this UI bundle were shipped out of sync.
export const EXPECTED_SCHEMA = 2;

// Fixed, developer-controlled command text. Never build this from user input;
// pass an argv array instead (it goes through execArgs with no shell).
export function kmgr(args) {
  if (Array.isArray(args)) return execArgs(KMGR, args);
  return exec(KMGR + ' ' + args);
}

let counter = 0;

// The child self-terminates after this many seconds. The UI transparently
// restarts it while visible, so a leaked monitor (app crash/force-stop, where
// the JS stop() never runs) can never outlive this window.
const MONITOR_LIFETIME = 30;

// A pkill pattern that cannot match its own shell: the command line of
// `pkill -f '[k]mgr-mon-...'` contains the brackets, not the literal tag.
function pkillPattern(tag) {
  return tag ? `[${tag[0]}]${tag.slice(1)}` : tag;
}

export { pkillPattern };

// Starts `kmgr monitor`, parses NDJSON and calls onSample per tick.
// Returns a stop() that also terminates the native child (the JS API has no
// kill, so we pkill a unique, bracket-escaped tag passed as an argv entry).
export function startMonitor(interval, onSample) {
  let stopped = false;
  let tag = '';
  let stopChild = null;

  function launch() {
    if (stopped) return;
    tag = `kmgr-mon-${(counter++).toString(36)}-${Date.now().toString(36)}`;
    stopChild = spawn(
      KMGR,
      ['monitor', '--interval', String(interval), '--seconds', String(MONITOR_LIFETIME), tag],
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
    if (hasKsu()) exec(`pkill -f '${pkillPattern(t)}'`).catch(() => {});
  };
}
