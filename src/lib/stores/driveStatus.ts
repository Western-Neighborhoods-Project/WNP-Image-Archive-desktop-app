import { writable, derived, type Readable } from 'svelte/store';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getDriveStatus, type DriveStatus } from '$lib/commands/drive';

// ── Drive status store ─────────────────────────────────────────────────────
// Single source of truth for the archive drive's mount state and stats.
// Fed by:
//   - one initial fetch via the get_drive_status command on app boot
//   - continuous Tauri "drive:status" events from the Rust poller
//
// Components (DriveIndicator, DriveDisconnectedScreen, GeneralPage) subscribe
// to this store and re-render when the backend pushes a new snapshot.

/** Initial (loading) state. Treat `connected: false` with no source_directory
 *  as the "haven't probed yet" state — the disconnect overlay logic skips
 *  rendering until we've heard back from the backend at least once. */
const INITIAL: DriveStatus = {
  connected: false,
  sourceDirectory: null,
  mountPoint: null,
  label: null,
  totalBytes: null,
  availableBytes: null,
  mountedAtMs: null,
  lastStatsAtMs: null,
  imageCount: null,
  formatMix: {},
};

export const driveStatus = writable<DriveStatus>(INITIAL);

/** True once the first event/fetch has populated the store. Used by
 *  `+page.svelte` to avoid flashing the disconnect overlay during the
 *  ~1s before the backend's first probe completes. */
export const driveStatusReady = writable<boolean>(false);

/** Derived flag: is the drive currently considered offline?
 *  Returns false (a) until the first probe lands and (b) when no source
 *  directory has been configured yet (e.g. during setup). The disconnect
 *  overlay only fires for "configured but unreachable", not for
 *  "not configured" — the setup screen handles the latter. */
export const driveDisconnected: Readable<boolean> = derived(
  [driveStatus, driveStatusReady],
  ([$status, $ready]) =>
    $ready && $status.sourceDirectory !== null && !$status.connected,
);

/** Initialize the store. Call once from `+page.svelte` onMount. Returns a
 *  cleanup function that unsubscribes from events — call it on destroy. */
export async function initDriveStatusListener(): Promise<() => void> {
  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen<DriveStatus>('drive:status', (event) => {
      driveStatus.set(event.payload);
      driveStatusReady.set(true);
    });
  } catch (e) {
    console.error('Failed to subscribe to drive:status events', e);
  }

  // Initial fetch — gives us state immediately rather than waiting for
  // the first poller tick (~0–1s).
  try {
    const initial = await getDriveStatus();
    driveStatus.set(initial);
    driveStatusReady.set(true);
  } catch (e) {
    console.error('Failed to fetch initial drive status', e);
    // Still mark ready so we don't hang the UI in "loading" forever.
    driveStatusReady.set(true);
  }

  return () => {
    if (unlisten) unlisten();
  };
}
