// Tauri auto-updater integration.
//
// Pairs with the GitHub Actions release workflow: when a tag is pushed,
// the workflow builds + signs the artifact and publishes a release with
// a `latest.json` manifest. This module handles the client side — check
// the manifest, prompt the user, download the new bundle, verify its
// signature against the embedded pubkey, replace the running app, and
// relaunch.
//
// Two entry points:
// - `checkForUpdates({ interactive: false })` — fires on app boot.
//   Silent if no update; only surfaces UI if one is available.
// - `checkForUpdates({ interactive: true })` — wired to the
//   "Check for updates…" menu item. Always shows feedback (either
//   "you're up to date" or the update prompt).

import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { ask, message } from '@tauri-apps/plugin-dialog';
import { writable, type Writable } from 'svelte/store';

export type UpdateStatus =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'downloading'; downloaded: number; total: number | null }
  | { kind: 'installing' };

export const updateStatus: Writable<UpdateStatus> = writable({ kind: 'idle' });

/** Bytes downloaded so far across the in-flight download — separate
 *  from `updateStatus` so progress UI can subscribe without re-rendering
 *  the entire status pattern on every chunk. */
export const updateBytes: Writable<{ downloaded: number; total: number | null }> =
  writable({ downloaded: 0, total: null });

let checkInFlight = false;

export async function checkForUpdates(opts: { interactive?: boolean } = {}) {
  if (checkInFlight) return;
  const interactive = opts.interactive ?? false;
  checkInFlight = true;
  updateStatus.set({ kind: 'checking' });

  let update: Update | null = null;
  try {
    update = await check();
  } catch (e) {
    updateStatus.set({ kind: 'idle' });
    checkInFlight = false;
    if (interactive) {
      await message(`Failed to check for updates:\n\n${e}`, {
        title: 'Update check failed',
        kind: 'warning',
      });
    } else {
      console.warn('Update check failed', e);
    }
    return;
  }

  if (!update) {
    updateStatus.set({ kind: 'idle' });
    checkInFlight = false;
    if (interactive) {
      await message("You're running the latest version.", {
        title: 'No updates available',
        kind: 'info',
      });
    }
    return;
  }

  // Update available. Always prompt — even on the silent boot check
  // we should ask before installing because the install relaunches the app.
  const notes = update.body?.trim() ?? '';
  const promptBody = notes
    ? `Version ${update.version} is available.\n\n${notes}\n\nInstall now? The app will relaunch.`
    : `Version ${update.version} is available.\n\nInstall now? The app will relaunch.`;
  const confirmed = await ask(promptBody, {
    title: 'Update available',
    kind: 'info',
    okLabel: 'Install',
    cancelLabel: 'Later',
  });
  if (!confirmed) {
    updateStatus.set({ kind: 'idle' });
    checkInFlight = false;
    return;
  }

  // Download + install. Stream progress through both stores.
  updateBytes.set({ downloaded: 0, total: null });
  updateStatus.set({ kind: 'downloading', downloaded: 0, total: null });
  let downloaded = 0;
  let total: number | null = null;

  try {
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          total = event.data.contentLength ?? null;
          downloaded = 0;
          updateBytes.set({ downloaded, total });
          updateStatus.set({ kind: 'downloading', downloaded, total });
          break;
        case 'Progress':
          downloaded += event.data.chunkLength;
          updateBytes.set({ downloaded, total });
          updateStatus.set({ kind: 'downloading', downloaded, total });
          break;
        case 'Finished':
          updateStatus.set({ kind: 'installing' });
          break;
      }
    });
  } catch (e) {
    updateStatus.set({ kind: 'idle' });
    checkInFlight = false;
    await message(`Update failed:\n\n${e}`, {
      title: 'Update error',
      kind: 'error',
    });
    return;
  }

  // Install succeeded. Relaunch into the new version — but if the relaunch
  // itself fails, the update is already applied, so tell the user to restart
  // manually rather than reporting a failure (which would send them to
  // re-download an update they already have).
  try {
    await relaunch();
  } catch (e) {
    updateStatus.set({ kind: 'idle' });
    checkInFlight = false;
    await message(
      `Version ${update.version} was installed, but the app couldn't restart automatically. Please quit and reopen Image Archive Manager to finish updating.\n\n${e}`,
      { title: 'Restart needed', kind: 'info' },
    );
  }
}
