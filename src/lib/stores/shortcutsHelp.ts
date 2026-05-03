import { writable } from 'svelte/store';

/** Open/closed state for the global keyboard-shortcuts cheat sheet.
 *  Triggered by `?` (anywhere) and the "Press ? for shortcuts" status-bar
 *  button. Mounted once in +page.svelte. */
export const shortcutsHelpOpen = writable<boolean>(false);

export function openShortcutsHelp(): void {
  shortcutsHelpOpen.set(true);
}

export function closeShortcutsHelp(): void {
  shortcutsHelpOpen.set(false);
}

export function toggleShortcutsHelp(): void {
  shortcutsHelpOpen.update((v) => !v);
}
