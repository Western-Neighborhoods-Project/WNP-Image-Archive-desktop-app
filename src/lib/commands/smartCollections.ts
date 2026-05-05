import { invoke } from '@tauri-apps/api/core';

/** Mirrors `smart_collections::SmartCollection`. The `filters` field is
 *  an opaque JSON string that the frontend serializes from FilterState
 *  on save and re-parses on apply — backend doesn't introspect it. */
export interface SmartCollection {
  id: number;
  name: string;
  filters: string; // JSON string
  createdAt: string;
}

export async function listSmartCollections(): Promise<SmartCollection[]> {
  return invoke('list_smart_collections');
}

export async function createSmartCollection(
  name: string,
  filters: string,
): Promise<SmartCollection> {
  return invoke('create_smart_collection', { name, filters });
}

export async function deleteSmartCollection(id: number): Promise<void> {
  return invoke('delete_smart_collection', { id });
}
