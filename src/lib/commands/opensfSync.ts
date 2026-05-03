import { invoke } from '@tauri-apps/api/core';
import type { ImageRecord } from './images';

/** Sync a single image's metadata from the OpenSFHistory API.
 *
 * Set `force=true` to bypass the 5-minute cache. Returns the updated
 * record (or the existing one if the cache is fresh / the API call
 * failed gracefully). Never throws on network errors — the backend
 * falls back to local data so the detail view keeps rendering. */
export async function syncImageFromOpensf(args: {
  imageId: number;
  force?: boolean;
}): Promise<ImageRecord> {
  return invoke('sync_image_from_opensf', {
    imageId: args.imageId,
    force: args.force ?? false,
  });
}
