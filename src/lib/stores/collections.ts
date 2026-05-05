import { writable } from 'svelte/store';
import type { Collection } from '$lib/commands/images';
import { invoke } from '@tauri-apps/api/core';

/** User-created collections (source='user'). Single source of truth shared across Sidebar, Grid, and DetailView. */
export const userCollections = writable<Collection[]>([]);

/** Re-fetch user collections from the backend and push into the store. Non-fatal. */
export async function refreshUserCollections(): Promise<void> {
  try {
    const all = await invoke<Collection[]>('get_collections');
    userCollections.set(all.filter((c) => c.source === 'user'));
  } catch (e) {
    console.error('refreshUserCollections failed:', e);
  }
}
