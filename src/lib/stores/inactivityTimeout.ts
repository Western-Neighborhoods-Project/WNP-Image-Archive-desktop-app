import { writable } from 'svelte/store';
import { getSetting, setSetting } from '$lib/commands/settings';

// ── Inactivity timeout (Plan 10) ───────────────────────────────────────────
//
// Lives in `app_settings` under key `inactivity_timeout_minutes`. Default
// 15. Read once on app boot via `loadInactivityTimeout`; the inactivity
// timer reads this value (via `get(...)`) on each reset, so changes from
// UsersPage take effect immediately.

const SETTING_KEY = 'inactivity_timeout_minutes';
const DEFAULT_MINUTES = 15;

export const inactivityTimeoutMinutes = writable<number>(DEFAULT_MINUTES);

export async function loadInactivityTimeout(): Promise<void> {
  try {
    const stored = await getSetting(SETTING_KEY);
    if (stored) {
      const n = parseInt(stored, 10);
      if (!Number.isNaN(n) && n > 0) {
        inactivityTimeoutMinutes.set(n);
      }
    }
  } catch (e) {
    console.error('Failed to load inactivity timeout setting', e);
  }
}

export async function saveInactivityTimeout(minutes: number): Promise<void> {
  if (!Number.isFinite(minutes) || minutes <= 0) {
    throw new Error('Timeout must be a positive number of minutes');
  }
  await setSetting(SETTING_KEY, String(Math.round(minutes)));
  inactivityTimeoutMinutes.set(Math.round(minutes));
}
