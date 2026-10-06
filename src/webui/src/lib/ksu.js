// Thin wrapper over the `kernelsu` npm library, with a browser mock fallback.
// The manager injects a global `ksu` object; the library calls into it.
import { exec as ksuExec, spawn as ksuSpawn, toast as ksuToast, enableEdgeToEdge as ksuEdgeToEdge } from 'kernelsu';
import { mockExec, mockSpawn } from './mock.js';

export function hasKsu() {
  return typeof window !== 'undefined' && typeof window.ksu !== 'undefined';
}

// exec(cmd) -> stdout string. Falls back to the mock when the API is absent.
export async function exec(cmd) {
  if (hasKsu()) {
    try {
      const r = await ksuExec(cmd);
      if (r && typeof r === 'object') return r.stdout ?? '';
      return String(r ?? '');
    } catch {
      return '';
    }
  }
  return mockExec(cmd);
}

// spawn(cmd, onLine, onExit) -> stop(). The kernelsu API has no kill; the
// caller terminates the process by matching a unique argument (see backend.js).
export function spawn(cmd, onLine, onExit) {
  if (hasKsu()) {
    try {
      const child = ksuSpawn(cmd, [], {});
      child.stdout.on('data', (data) => {
        String(data)
          .split('\n')
          .forEach((line) => {
            const t = line.replace(/\r$/, '');
            if (t) onLine(t);
          });
      });
      if (onExit && typeof child.on === 'function') {
        child.on('exit', (code) => onExit(code));
      }
      return () => {};
    } catch {
      /* fall through to mock */
    }
  }
  return mockSpawn(cmd, onLine);
}

export function toast(message) {
  if (hasKsu()) {
    try {
      ksuToast(message);
    } catch {
      /* ignore */
    }
  }
}

// Let the page fill the system bars; the manager then injects --safe-area-*
// so our black background covers the status bar.
export function enableEdgeToEdge(enable) {
  if (hasKsu()) {
    try {
      ksuEdgeToEdge(enable);
    } catch {
      /* ignore */
    }
  }
}
