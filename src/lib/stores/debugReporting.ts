import { writable } from 'svelte/store';
import { getPublicSetting } from '$lib/commands/settings';

/**
 * Whether in-app bug reporting is turned on (Settings → Debugging).
 * Gates the sidebar bug icon and the ⌘⇧B shortcut for every logged-in
 * user. Hydrated post-login by `loadDebugReporting()`; DebuggingPage
 * writes it directly when the admin flips the toggle so the icon
 * appears/disappears without a reload.
 */
export const debugReportingEnabled = writable<boolean>(false);

/** Whether the report dialog is open. Set by the sidebar icon and ⌘⇧B. */
export const bugReportOpen = writable<boolean>(false);

/** Hydrate from settings. Call once a session exists — the setting is
 *  public but still requires login to read. */
export async function loadDebugReporting(): Promise<void> {
  try {
    const value = await getPublicSetting('debug_reporting_enabled');
    debugReportingEnabled.set(value === 'true');
  } catch (e) {
    console.error('Failed to load debug_reporting_enabled', e);
  }
}
