import { writable, derived, type Readable } from 'svelte/store';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  getBackgroundProgress,
  type BackgroundProgress,
} from '$lib/commands/backgroundJobs';

// ── Background progress store ──────────────────────────────────────────────
// Single source of truth for the footer activity indicator. Fed by:
//   - one initial fetch (get_background_progress) on app boot
//   - `background:progress` Tauri events from the Plan 13 worker
//
// Components subscribe to this store + the derived booleans below.

const INITIAL: BackgroundProgress = {
  thumbnails: { pending: 0, done: 0, failed: 0 },
  metadata: { pending: 0, done: 0, failed: 0 },
  images: { total: 0, resolved: 0, pending: 0 },
  busy: false,
};

export const backgroundProgress = writable<BackgroundProgress>(INITIAL);

/** True if anything is pending or actively processing. Drives the
 *  spinner state on the footer pill. */
export const isProcessing: Readable<boolean> = derived(
  backgroundProgress,
  ($p) => $p.busy || $p.images.pending > 0,
);

/** Total failures across both job types — used by the pill to flip
 *  destructive when non-zero. */
export const totalFailures: Readable<number> = derived(
  backgroundProgress,
  ($p) => $p.thumbnails.failed + $p.metadata.failed,
);

/** Initialize the store. Call once from `+page.svelte` onMount.
 *  Returns a cleanup function. */
export async function initBackgroundProgressListener(): Promise<() => void> {
  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen<BackgroundProgress>(
      'background:progress',
      (event) => {
        backgroundProgress.set(event.payload);
      },
    );
  } catch (e) {
    console.error('Failed to subscribe to background:progress', e);
  }

  // Initial pull so the indicator has a baseline before the first event.
  try {
    const initial = await getBackgroundProgress();
    backgroundProgress.set(initial);
  } catch (e) {
    // Pre-login this throws ("Not logged in") — fine, the worker hasn't
    // emitted anything either. State stays at INITIAL until the user
    // logs in and the worker's next tick fires.
    console.debug('Background progress initial fetch skipped', e);
  }

  return () => {
    if (unlisten) unlisten();
  };
}
