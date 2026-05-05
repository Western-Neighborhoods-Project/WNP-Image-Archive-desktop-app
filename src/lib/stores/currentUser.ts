import { writable, derived, type Readable } from 'svelte/store';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  getCurrentUser,
  isSetupRequired,
  type UserRole,
  type UserSession,
} from '$lib/commands/auth';

// ── Auth state stores ──────────────────────────────────────────────────────
//
// `currentUser` is the active session (null when logged out).
// `setupRequired` is null while we're probing on boot, then true (no users
// exist — bootstrap admin) or false (regular login).
//
// Both are populated by `initAuthListener()` which subscribes to the Rust
// side's `auth:changed` events.

export const currentUser = writable<UserSession | null>(null);
export const setupRequired = writable<boolean | null>(null);

export const currentUserRole: Readable<UserRole | null> = derived(
  currentUser,
  ($u) => $u?.role ?? null,
);

export const isAdmin: Readable<boolean> = derived(
  currentUserRole,
  ($r) => $r === 'admin',
);

/** True once both initial probes have returned — used by `+page.svelte`
 *  to avoid flashing wrong UI during the brief boot window. */
export const authReady = writable<boolean>(false);

/** Initialize. Call once from `+page.svelte` onMount. Returns cleanup fn. */
export async function initAuthListener(): Promise<() => void> {
  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen<UserSession | null>('auth:changed', (event) => {
      currentUser.set(event.payload);
      // Whenever the session changes, setup_required can flip too
      // (creating the first admin makes it false). Re-check.
      isSetupRequired().then((b) => setupRequired.set(b)).catch(() => {});
    });
  } catch (e) {
    console.error('auth:changed listen failed', e);
  }

  // Probe initial state.
  try {
    const [user, needsSetup] = await Promise.all([
      getCurrentUser(),
      isSetupRequired(),
    ]);
    currentUser.set(user);
    setupRequired.set(needsSetup);
  } catch (e) {
    console.error('Failed to probe auth state', e);
    setupRequired.set(false); // safest fallback
  } finally {
    authReady.set(true);
  }

  return () => {
    if (unlisten) unlisten();
  };
}
