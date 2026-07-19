/// Shared data types passed between the Rust backend and the TypeScript frontend.
/// All types derive serde Serialize/Deserialize for JSON transport via Tauri invoke.
use serde::{Deserialize, Serialize};

// ============================================================
// Core Image Record
// ============================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageRecord {
    pub id: i64,
    pub file_path: String,
    pub catalog_number: String,
    pub file_size: Option<i64>,
    pub file_modified: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub keywords: Option<String>, // JSON array string: ["kw1","kw2"]
    pub date_display: Option<String>,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    pub photographer: Option<String>,
    pub donor: Option<String>,
    pub acquisition_date: Option<String>,
    pub archival_collection: Option<String>,
    pub usage_rights: Option<String>,
    pub internal_notes: Option<String>,
    pub thumbnail_path: Option<String>,
    pub thumbnail_generated: bool,
    pub metadata_synced: bool,
    // Plan 9: OpenSFHistory mirror columns
    pub caption: Option<String>,
    pub dimensions: Option<String>,
    pub format: Option<String>,
    pub publisher: Option<String>,
    pub citation: Option<String>,
    pub download_permitted: Option<i64>, // 0/1; null if never synced
    pub neighborhoods: Option<String>,    // JSON array of slugs
    pub photosets: Option<String>,        // JSON object {id: title}
    pub osf_collections: Option<String>,  // JSON array of names
    pub osf_page_url: Option<String>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================
// Scanner
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files: u64,
    pub new_files: u64,
    pub archive_collections_found: u64,
    pub scan_duration_ms: u64,
    /// Number of `walkdir` entries we couldn't read (permission denied,
    /// IO error, etc). Surfaced to the UI so users notice when a chunk
    /// of the directory was silently skipped.
    pub walk_errors: u64,
    /// Plan 12: id of the source directory the scan was associated with.
    /// Frontend uses this to refresh the sidebar tree afterwards.
    pub source_directory_id: i64,
}

// ============================================================
// Source Directories (Plan 12)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDirectory {
    pub id: i64,
    pub path: String,
    pub label: String,
    pub created_at: String,
    pub image_count: i64,
}

/// One node in the sidebar's source-directory tree. The root nodes are
/// source directories themselves; children are subfolders inside them
/// (computed on demand from distinct `relative_dir` values on images).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTreeNode {
    /// `null` at the root level (we use the parent SourceDirectory's id);
    /// otherwise the source the node lives in.
    pub source_directory_id: i64,
    /// Display label — last path segment, or the source's label at depth 0.
    pub label: String,
    /// `relative_dir` value to use when filtering images for this node.
    /// Empty string at the source root; `Forest Hill/1995-batch` for a
    /// nested folder.
    pub relative_dir: String,
    pub image_count: i64,
    pub children: Vec<SourceTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTreeRoot {
    pub source: SourceDirectory,
    pub children: Vec<SourceTreeNode>,
}

// ============================================================
// Background jobs (Plan 13)
// ============================================================

/// Counts for the footer indicator. Both `thumbnails` and `metadata`
/// share the same `JobStateCounts` shape — the worker emits this on
/// every batch tick.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStateCounts {
    pub pending: i64,
    pub done: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageProgress {
    pub total: i64,
    /// Images where BOTH thumbnail and metadata states are no longer
    /// `pending` (i.e. each is either `done` or `failed`).
    pub resolved: i64,
    /// Images where EITHER state is still `pending`. Used as the
    /// remaining-count for the footer pill.
    pub pending: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundProgress {
    /// Per-job-type counts (used by the failures popover for per-tab
    /// "Retry" buttons that operate on one queue at a time).
    pub thumbnails: JobStateCounts,
    pub metadata: JobStateCounts,
    /// Per-image rollup. The footer indicator's progress bar uses this
    /// so 82 images shows as `N / 82`, not `N / 164` (which would
    /// double-count images that have both a thumbnail job and a
    /// metadata job).
    pub images: ImageProgress,
    /// True when the worker is currently mid-batch. Footer can show a
    /// spinner instead of the "ready" state.
    pub busy: bool,
}

/// One row in the failures popover. Per-file error visibility was a
/// Plan 13 ask — the user should be able to see *why* a particular
/// thumbnail or metadata extraction failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureRecord {
    pub image_id: i64,
    pub catalog_number: String,
    pub file_path: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanStats {
    pub total_images: u64,
    pub images_with_thumbnails: u64,
    pub images_without_metadata: u64,
}

// ============================================================
// Metadata
// ============================================================

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ExtractedMetadata {
    pub file_path: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub date_start: Option<String>,
    pub photographer: Option<String>,
    pub usage_rights: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataImportResult {
    pub processed: u64,
    pub updated: u64,
    pub errors: u64,
    pub duration_ms: u64,
}

// ============================================================
// Thumbnails
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailResult {
    pub extracted: u64,
    pub fallback_generated: u64,
    pub failed: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailRequest {
    pub image_ids: Vec<i64>,
}

// ============================================================
// Queries
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageQuery {
    pub offset: u64,
    pub limit: u64,
    pub sort_by: Option<String>,    // "catalog_number" | "date_start" | "created_at" | "updated_at"
    pub sort_order: Option<String>, // "asc" | "desc"
    // Filters
    pub city: Option<String>,
    pub photographer: Option<String>,
    pub collection_id: Option<i64>,
    pub year_start: Option<i32>,
    pub year_end: Option<i32>,
    pub missing_metadata: Option<bool>,
    pub search_query: Option<String>,
    // Plan 12: source-directory tree filters. source_directory_id alone
    // restricts to the entire tree under that source; combine with
    // relative_dir to scope to a specific subfolder + its descendants.
    pub source_directory_id: Option<i64>,
    pub relative_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageQueryResult {
    pub images: Vec<ImageRecord>,
    pub total_count: u64,
}

// ============================================================
// Metadata Editing
// ============================================================

/// A single field change, used for audit trail and metadata updates.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldChange {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Batch metadata update payload from the frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataUpdate {
    pub image_id: i64,
    pub changes: Vec<FieldChange>,
}

/// A single audit log entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub image_id: i64,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
    pub changed_at: String,
}

/// A single recent activity entry for the sidebar ActivityCard.
/// Joined view of audit_log + images so the card can display the
/// catalog number alongside the field that changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentActivityEntry {
    pub id: i64,
    pub changed_by: String,
    pub catalog_number: String,
    pub field_name: String,
    pub new_value: Option<String>,
    pub changed_at: String,
}

/// A single audit log entry, joined with the images table for the
/// catalog number. Returned by the global audit-log query that powers
/// the Audit log view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogGlobalEntry {
    pub id: i64,
    pub image_id: i64,
    pub catalog_number: String,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
    pub changed_at: String,
}

// ============================================================
// Filter Options
// ============================================================

/// Distinct values used to populate filter dropdowns.
#[derive(Debug, Serialize, Deserialize)]
pub struct FilterOptions {
    pub cities: Vec<String>,
    pub photographers: Vec<String>,
    pub year_min: Option<i32>,
    pub year_max: Option<i32>,
}

// ============================================================
// Collections
// ============================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub source: String, // "user" | "archive"
    pub description: Option<String>,
    pub image_count: u64,
    pub created_at: String,
}

// ============================================================
// Image Requests (Phase 4)
// ============================================================

/// A single line item inside an order.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItem {
    pub catalog_number: String,
    pub title: Option<String>,
    pub resolution: String, // "high" | "medium" | "low"
    pub price_cents: i64,
    pub price: f64,
}

/// A customer order from the Laravel API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    /// Stable machine identifier — used for fulfill/fail API calls.
    pub uuid: String,
    /// Human-friendly order identifier from the OpenSFHistory API.
    /// Always present; uuid is for machine interactions, order_number for display.
    pub order_number: String,
    pub name: String,
    pub email: String,
    // Laravel order status: pending (unpaid) | paid (awaiting fulfillment) |
    // processing | completed | failed | cancelled | refunded.
    pub status: String,
    pub total_cents: i64,
    pub total: f64,
    pub currency: String,
    pub item_count: i64,
    pub created_at: String,
    pub paid_at: Option<String>,
    pub items: Vec<OrderItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrdersMeta {
    pub total: i64,
    pub fulfillable: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrdersResponse {
    pub data: Vec<Order>,
    pub meta: OrdersMeta,
}

/// Result returned to the frontend after a fulfill/fail action.
#[derive(Debug, Serialize, Deserialize)]
pub struct FulfillResult {
    pub uuid: String,
    pub zip_url: String,
    pub items_fulfilled: usize,
}

/// Result returned to the frontend after `create_share_link` succeeds.
/// Used to drive the success state in the share dialog.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateShareLinkResult {
    pub image_url: String,
    pub recipient_email: String,
    pub resolution_label: String,
}
