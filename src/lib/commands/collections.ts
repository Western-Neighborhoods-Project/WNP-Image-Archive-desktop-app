import { invoke } from '@tauri-apps/api/core';
import type { Collection } from '$lib/commands/images';

export type { Collection };

export async function createCollection(name: string): Promise<number> {
  return invoke<number>('create_collection', { name });
}

export async function renameCollection(id: number, name: string): Promise<void> {
  return invoke<void>('rename_collection', { id, name });
}

export async function deleteCollection(id: number): Promise<void> {
  return invoke<void>('delete_collection', { id });
}

export async function addToCollection(collectionId: number, imageIds: number[]): Promise<void> {
  return invoke<void>('add_to_collection', { collectionId, imageIds });
}

export async function removeFromCollection(
  collectionId: number,
  imageIds: number[]
): Promise<void> {
  return invoke<void>('remove_from_collection', { collectionId, imageIds });
}

export async function getImageCollections(imageId: number): Promise<Collection[]> {
  return invoke<Collection[]>('get_image_collections', { imageId });
}
