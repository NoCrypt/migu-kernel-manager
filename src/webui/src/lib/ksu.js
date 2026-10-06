// Thin wrapper over the `kernelsu` npm library, with a browser mock fallback.
// The manager injects a global `ksu` object; the library calls into it.
import { exec as ksuExec, spawn as ksuSpawn, toast as ksuToast, enableEdgeToEdge as ksuEdgeToEdge } from 'kernelsu';
import { mockExec, mockSpawn } from './mock.js';

export function hasKsu() {
  return typeof window !== 'undefined' && typeof window.ksu !== 'undefined';
}

// exec(cmd) -> stdout string. Falls back to the mock when the API is absent.
// NOTE: exec() takes a shell string, so never interpolate a user-supplied value
// into it. Use execArgs(cmd, args) for anything that may contain spaces/quotes/;.
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

// execArgs(cmd, args) -> stdout string, passed to the manager as an argv array
// with no shell involved. Required for user-supplied values (tunable values,
// custom paths, property values, ...).
export function execArgs(cmd, args) {
  if (hasKsu()) {
    return new Promise((resolve) => {
      let out = '';
      try {
        const child = ksuSpawn(cmd, args, {});
        child.stdout.on('data', (data) => {
          out += String(data);
        });
        if (typeof child.on === 'function') child.on('exit', () => resolve(out));
        else resolve(out);
        child.on('error', () => resolve(out));
      } catch {
        resolve('');
      }
    });
  }
  return mockExec([cmd, ...args].join(' '));
}

// spawn(cmd, args, onLine, onExit) -> stop(). args is an argv array; the child
// process is terminated by pkill on a bracket-escaped tag (see backend.js).
export function spawn(cmd, args, onLine, onExit) {
  const argv = args || [];
  if (hasKsu()) {
    try {
      const child = ksuSpawn(cmd, argv, {});
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
  return mockSpawn([cmd, ...argv].join(' '), onLine);
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
