import { writable } from 'svelte/store';

/**
 * Open/closed state for the global ⌘K command palette. The CommandBar
 * component is mounted once in the root layout; toggling this store
 * shows/hides it. `query` resets to an empty string every time the bar
 * opens so each launch starts clean.
 */
export const commandBarOpen = writable<boolean>(false);
export const commandBarQuery = writable<string>('');

export function openCommandBar(): void {
  commandBarQuery.set('');
  commandBarOpen.set(true);
}

export function closeCommandBar(): void {
  commandBarOpen.set(false);
}
