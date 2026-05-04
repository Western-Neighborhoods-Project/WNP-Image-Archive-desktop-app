import { invoke } from '@tauri-apps/api/core';

/** Mirrors `models::JobStateCounts`. */
export interface JobStateCounts {
  pending: number;
  done: number;
  failed: number;
}

/** Mirrors `models::ImageProgress`. Per-image rollup: an image is
 *  `resolved` when both its thumbnail and metadata states are no longer
 *  `pending`; `pending` when either still is. */
export interface ImageProgress {
  total: number;
  resolved: number;
  pending: number;
}

/** Mirrors `models::BackgroundProgress`. */
export interface BackgroundProgress {
  thumbnails: JobStateCounts;
  metadata: JobStateCounts;
  images: ImageProgress;
  /** True when the worker is mid-batch. */
  busy: boolean;
}

/** One row in the failures popover. */
export interface FailureRecord {
  imageId: number;
  catalogNumber: string;
  filePath: string;
  error: string | null;
}

export async function getBackgroundProgress(): Promise<BackgroundProgress> {
  return invoke('get_background_progress');
}

export async function listThumbnailFailures(limit = 50): Promise<FailureRecord[]> {
  return invoke('list_thumbnail_failures', { limit });
}

export async function listMetadataFailures(limit = 50): Promise<FailureRecord[]> {
  return invoke('list_metadata_failures', { limit });
}

/** Flips every failed thumbnail back to pending. Returns count flipped. */
export async function retryFailedThumbnails(): Promise<number> {
  return invoke('retry_failed_thumbnails');
}

export async function retryFailedMetadata(): Promise<number> {
  return invoke('retry_failed_metadata');
}
