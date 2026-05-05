import { writable } from 'svelte/store';
import {
  listSmartCollections,
  type SmartCollection,
} from '$lib/commands/smartCollections';

export const smartCollections = writable<SmartCollection[]>([]);

export async function refreshSmartCollections(): Promise<void> {
  try {
    const list = await listSmartCollections();
    smartCollections.set(list);
  } catch (e) {
    console.error('Failed to load smart collections', e);
  }
}
