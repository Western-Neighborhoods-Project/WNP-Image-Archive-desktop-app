import { invoke } from "@tauri-apps/api/core";

export interface RecentActivityEntry {
  id: number;
  changed_by: string;
  catalog_number: string;
  field_name: string;
  new_value: string | null;
  changed_at: string;
}

export async function getRecentActivity(
  limit = 5,
): Promise<RecentActivityEntry[]> {
  return invoke<RecentActivityEntry[]>("get_recent_activity", { limit });
}

// ============================================================
// Global audit log (Plan 4)
// ============================================================

export interface AuditLogGlobalEntry {
  id: number;
  image_id: number;
  catalog_number: string;
  field_name: string;
  old_value: string | null;
  new_value: string | null;
  changed_by: string;
  changed_at: string;
}

export interface AuditLogFilter {
  /** Restrict to a single field name (e.g. 'city'); null/undefined = all fields. */
  fieldName?: string | null;
  /** SQLite datetime string ('YYYY-MM-DD HH:MM:SS'); null = no lower bound. */
  since?: string | null;
  /** SQLite datetime string; null = no upper bound. */
  until?: string | null;
  /** Default 100. */
  limit?: number;
  /** Default 0. */
  offset?: number;
}

export async function getAuditLogGlobal(
  filter: AuditLogFilter = {},
): Promise<AuditLogGlobalEntry[]> {
  return invoke<AuditLogGlobalEntry[]>("get_audit_log_global", {
    fieldName: filter.fieldName ?? null,
    since: filter.since ?? null,
    until: filter.until ?? null,
    limit: filter.limit ?? 100,
    offset: filter.offset ?? 0,
  });
}

/**
 * Export audit log entries to a CSV file. The caller picks the path via
 * `@tauri-apps/plugin-dialog`'s `save()` and passes it here. Returns the
 * number of rows written.
 */
export async function exportAuditLogCsv(opts: {
  fieldName: string | null;
  since: string | null;
  until: string | null;
  path: string;
}): Promise<number> {
  return invoke<number>("export_audit_log_csv", opts);
}
