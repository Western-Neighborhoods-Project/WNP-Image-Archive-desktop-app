// Inactivity timer (Plan 10).
//
// Watches user activity at the document level and fires `onExpired` after
// `getTimeoutMs()` ms of no activity. Activity = mousemove, mousedown,
// keydown, touchstart, scroll. Reset is throttled to once every 5s so we
// don't burn CPU resetting the timer 60 times per second during mouse
// movement.
//
// `getTimeoutMs` is a function (not a value) so the caller can change the
// timeout setting (e.g. via UsersPage) without re-installing the timer.

export interface InactivityTimerOptions {
  getTimeoutMs: () => number;
  onExpired: () => void;
}

const ACTIVITY_EVENTS: (keyof DocumentEventMap)[] = [
  'mousemove',
  'mousedown',
  'keydown',
  'touchstart',
  'scroll',
];

const RESET_THROTTLE_MS = 5_000;

export function installInactivityTimer(
  opts: InactivityTimerOptions,
): () => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  let lastReset = 0;

  function scheduleExpiry() {
    if (timeout !== null) clearTimeout(timeout);
    const ms = opts.getTimeoutMs();
    if (ms <= 0) return; // 0 or negative = timer disabled
    timeout = setTimeout(() => {
      timeout = null;
      try {
        opts.onExpired();
      } catch (e) {
        console.error('inactivity onExpired threw', e);
      }
    }, ms);
  }

  function onActivity() {
    const now = Date.now();
    if (now - lastReset < RESET_THROTTLE_MS) return;
    lastReset = now;
    scheduleExpiry();
  }

  for (const evt of ACTIVITY_EVENTS) {
    document.addEventListener(evt, onActivity, { passive: true });
  }
  scheduleExpiry();

  return () => {
    for (const evt of ACTIVITY_EVENTS) {
      document.removeEventListener(evt, onActivity);
    }
    if (timeout !== null) clearTimeout(timeout);
  };
}
