# Plan 4 — Audit log view

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 4 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (`PageHeader`, `StatusBar`, `Kbd` primitives).

## Goal

Replace `AuditLogStub` with a real audit-log view — every metadata change, approval, import, with diffs and filters.

## Scope

- New `AuditLogView.svelte` with: page header + filter bar (user/date/field) + grouped-by-date list with date stickies + per-entry diff display (red strike-through old → green new) + "View image ↗" link
- New backend command: `get_audit_log_global(filter, limit, offset)` (we have per-image `get_audit_log` and Plan-1's `get_recent_activity`; this is a third variant supporting filtering)
- CSV export button → Rust generates and prompts save dialog

## Out of scope

- Undo from audit log (keep it read-only)
- Full-text search inside diffs (filter is enough for now)

## Key files

**New:**
- `src/lib/components/audit/AuditLogView.svelte`, `AuditEntryRow.svelte`, `DateGroup.svelte`

**Modify:**
- `src-tauri/src/editor.rs` (new global query + CSV export command)
- `src-tauri/src/lib.rs` (register new commands)
- `src/lib/commands/activity.ts` (add `getAuditLogGlobal` + `exportAuditLogCsv`)
- `src/routes/+page.svelte` (swap `AuditLogStub` for `AuditLogView`)

## Open questions

- Date filter UI: calendar popover or simple dropdown (Last 7d / 30d / 90d / All)?
- CSV export columns: same as visible columns, or full row data?
- Pagination strategy (infinite scroll vs paged)?
- Single-user means `changed_by` is uniform — drop the user filter or keep for future-proofing?

## Verification

Make several edits across multiple images, navigate to Audit log, confirm entries appear in reverse chrono with date groupings. Filter by field, confirm filter works. Export CSV, open in spreadsheet. Visual diff against artboard 4b.

## Estimated size

**S–M.**
