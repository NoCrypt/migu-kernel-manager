// Live Monitor child management: one `kmgr live <tab>` process at a time.
// NDJSON on stdout; the child self-terminates after LIFETIME seconds and the
// UI restarts it, so a leaked child can never outlive the watchdog.
import { spawn, exec, hasKsu } from './ksu.js';
import { KMGR } from './backend.js';

let counter = 0;
const LIFETIME = 30;

export function startStream(args, onLine, onGone) {
  let stopped = false;
  let tag = '';
  let child = null;

  function launch() {
    if (stopped) return;
    tag = `kmgr-live-${(counter++).toString(36)}-${Date.now().toString(36)}`;
    child = spawn(
      `${KMGR} ${args} --seconds ${LIFETIME} ${tag}`,
      (line) => {
        let obj;
        try {
          obj = JSON.parse(line);
        } catch {
          return;
        }
        if (obj.gone) {
          if (onGone) onGone();
          return;
        }
        onLine(obj);
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
      child && child();
    } catch {
      /* ignore */
    }
    if (hasKsu()) exec(`pkill -f ${t}`).catch(() => {});
  };
}

// Mount helper for a tab component: starts on attach(), stops on detach(),
// and pauses while the page is hidden (visibilitychange).
export function mountLive(getArgs, onLine, onGone) {
  let stop = null;
  const start = () => {
    if (!stop) stop = startStream(getArgs(), onLine, onGone);
  };
  const halt = () => {
    if (stop) {
      stop();
      stop = null;
    }
  };
  const onVis = () => (document.hidden ? halt() : start());
  return {
    start,
    halt,
    restart() {
      halt();
      start();
    },
    attach() {
      start();
      document.addEventListener('visibilitychange', onVis);
    },
    detach() {
      halt();
      document.removeEventListener('visibilitychange', onVis);
    }
  };
}
