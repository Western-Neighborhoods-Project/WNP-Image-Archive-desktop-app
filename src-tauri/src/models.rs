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
    pub uuid: String,
    pub name: String,
    pub email: String,
    pub status: String, // "pending" | "fulfilled" | "failed"
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
