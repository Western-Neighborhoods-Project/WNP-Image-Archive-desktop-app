import { invoke } from '@tauri-apps/api/core';

/** Mirrors `drive::DriveStatus` in `src-tauri/src/drive.rs`. */
export interface DriveStatus {
  connected: boolean;
  sourceDirectory: string | null;
  /** `/Volumes/<name>` if the source directory is on an external volume,
   *  null when it lives on internal storage (in which case there's no
   *  separate "drive" concept — but we still monitor the path). */
  mountPoint: string | null;
  /** Pretty drive label (basename of mountPoint). */
  label: string | null;
  totalBytes: number | null;
  availableBytes: number | null;
  /** When the drive was first detected as mounted in this session. */
  mountedAtMs: number | null;
  /** When the stats fields above were last refreshed. */
  lastStatsAtMs: number | null;
  imageCount: number | null;
  /** e.g. `{ jpg: 12000, tiff: 5000 }`. */
  formatMix: Record<string, number>;
}

/** Returns the cached snapshot. Cheap; doesn't probe disk. */
export async function getDriveStatus(): Promise<DriveStatus> {
  return invoke('get_drive_status');
}

/** Forces an immediate re-probe + stats refresh. Used by the disconnect
 *  overlay's Retry button. */
export async function retryDriveConnection(): Promise<DriveStatus> {
  return invoke('retry_drive_connection');
}

/** Reveal the drive in Finder. Falls back to source_directory if the
 *  drive isn't on a /Volumes mount. */
export async function revealDriveInFinder(): Promise<void> {
  return invoke('reveal_drive_in_finder');
}
