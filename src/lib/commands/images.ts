import { invoke } from '@tauri-apps/api/core';

export interface ImageRecord {
  id: number;
  file_path: string;
  catalog_number: string;
  file_size: number | null;
  file_modified: string | null;
  title: string | null;
  description: string | null;
  city: string | null;
  state: string | null;
  country: string | null;
  keywords: string | null; // JSON array string: '["kw1","kw2"]'
  date_display: string | null;
  date_start: string | null;
  date_end: string | null;
  photographer: string | null;
  donor: string | null;
  acquisition_date: string | null;
  archival_collection: string | null;
  usage_rights: string | null;
  internal_notes: string | null;
  thumbnail_path: string | null;
  thumbnail_generated: boolean;
  metadata_synced: boolean;
  // Plan 9: OpenSFHistory mirror columns
  caption: string | null;
  dimensions: string | null;
  format: string | null;
  publisher: string | null;
  citation: string | null;
  download_permitted: number | null; // 0/1; null if never synced
  neighborhoods: string | null; // JSON array as string
  photosets: string | null; // JSON object as string
  osf_collections: string | null; // JSON array as string
  osf_page_url: string | null;
  last_synced_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ImageQuery {
  offset: number;
  limit: number;
  sort_by?: string;
  sort_order?: 'asc' | 'desc';
  // Filters
  city?: string | null;
  photographer?: string | null;
  collection_id?: number | null;
  year_start?: number | null;
  year_end?: number | null;
  missing_metadata?: boolean | null;
  search_query?: string | null;
}

export interface ImageQueryResult {
  images: ImageRecord[];
  total_count: number;
}

export interface ScanResult {
  total_files: number;
  new_files: number;
  archive_collections_found: number;
  scan_duration_ms: number;
}

export interface ScanStats {
  total_images: number;
  images_with_thumbnails: number;
  images_without_metadata: number;
}

export interface MetadataImportResult {
  processed: number;
  updated: number;
  errors: number;
  duration_ms: number;
}

export interface ThumbnailResult {
  extracted: number;
  fallback_generated: number;
  failed: number;
  duration_ms: number;
}

export async function queryImages(query: ImageQuery): Promise<ImageQueryResult> {
  return invoke('query_images', { query });
}

export async function getImage(id: number): Promise<ImageRecord> {
  return invoke('get_image', { id });
}

export async function scanDirectory(path: string): Promise<ScanResult> {
  return invoke('scan_directory', { path });
}

export async function getScanStats(): Promise<ScanStats> {
  return invoke('get_scan_stats');
}

export async function extractMetadataBatch(directory: string): Promise<MetadataImportResult> {
  return invoke('extract_metadata_batch', { directory });
}

export async function extractExifThumbnailsBatch(): Promise<ThumbnailResult> {
  return invoke('extract_exif_thumbnails_batch');
}

export async function generateFullThumbnails(imageIds: number[]): Promise<ThumbnailResult> {
  return invoke('generate_full_thumbnails', { request: { image_ids: imageIds } });
}

// ============================================================
// Phase 2: Metadata Editing
// ============================================================

export interface FieldChange {
  field: string;
  old_value: string | null;
  new_value: string | null;
}

export interface MetadataUpdate {
  image_id: number;
  changes: FieldChange[];
}

export interface AuditLogEntry {
  id: number;
  image_id: number;
  field_name: string;
  old_value: string | null;
  new_value: string | null;
  changed_by: string;
  changed_at: string;
}

export interface FilterOptions {
  cities: string[];
  photographers: string[];
  year_min: number | null;
  year_max: number | null;
}

export async function updateImageMetadata(update: MetadataUpdate): Promise<void> {
  return invoke('update_image_metadata', { update });
}

export async function getAuditLog(imageId: number): Promise<AuditLogEntry[]> {
  return invoke('get_audit_log', { imageId });
}

/** Write DB metadata back into the image file via ExifTool. Non-fatal — throws on failure. */
export async function writeMetadataToFile(imageId: number): Promise<void> {
  return invoke('write_metadata_to_file', { imageId });
}

export async function logImageView(imageId: number): Promise<void> {
  return invoke('log_image_view', { imageId });
}

export async function getRecentlyViewed(): Promise<ImageRecord[]> {
  return invoke('get_recently_viewed');
}

export async function getFilterOptions(): Promise<FilterOptions> {
  return invoke('get_filter_options');
}

// ============================================================
// Collections
// ============================================================

export interface Collection {
  id: number;
  name: string;
  source: 'archive' | 'user';
  description: string | null;
  image_count: number;
  created_at: string;
}

export async function getCollections(): Promise<Collection[]> {
  return invoke('get_collections');
}
