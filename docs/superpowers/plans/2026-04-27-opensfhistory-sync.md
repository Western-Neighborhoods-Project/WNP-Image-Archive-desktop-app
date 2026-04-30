# Plan 9 — OpenSFHistory metadata sync

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 9 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (primitives). Probably wants Plan 7 (inbox) shipped so newly-imported images can sync from OpenSFHistory once metadata is filled in there.

## Goal

Pull metadata from OpenSFHistory (Laravel) so the local catalog stays current with the public archive's source-of-truth records, and push local edits back. After this plan, metadata edited on either side stays in sync.

## Scope

- Backend: poll OpenSFHistory metadata API (delta endpoint? full-list?) on configurable interval
- Bidirectional sync: pull (OpenSFHistory → local) + push (local edits → OpenSFHistory) on field-blur save (current pattern)
- Conflict resolution: OpenSFHistory wins for shared fields; local-only fields (`internal_notes`, etc.) never sync
- UI: `SyncStatus.svelte` indicator in status bar; conflict resolution prompts on rare collisions
- Settings: API key, sync interval, dry-run/verify mode
- Audit log entries record sync source ("synced from OpenSFHistory" vs "edit pushed to OpenSFHistory")

## Out of scope

- Image upload to OpenSFHistory (separate consideration — OpenSF currently has its own image library; this sync is metadata-only)
- User-level auth on OpenSFHistory (assume single API key with full edit access)

## Key files

**New:**
- `src-tauri/src/opensf_sync.rs` (HTTP client, delta logic, conflict resolution)
- `src/lib/components/sync/SyncStatus.svelte`, `ConflictDialog.svelte`

**Modify:**
- `src-tauri/src/editor.rs` (`update_image_metadata` adds post-update push hook)
- `src-tauri/src/lib.rs`
- `src-tauri/src/db.rs` (add `last_synced_at`, `opensf_id` columns to images table — schema migration)
- Settings sub-page "OpenSFHistory API" (Plan 2 created stub) with credentials + interval

## Open questions

- **OpenSFHistory API surface: what endpoints exist, what's the schema, pagination, rate limits — need API docs from user before this plan can be fully expanded.** This is the biggest unknown and should be resolved at the start of brainstorming, not at implementation time.
- Sync interval (5min? 15min? on-demand only?)
- Conflict UI (rare but needs thought) — show diff, let user pick, or always defer to OpenSFHistory?
- Field mapping (local `catalog_number` ↔ OpenSF identifier)
- Initial sync for an existing populated DB — migration strategy
- How sync interacts with the import inbox (Plan 7) — newly-imported images probably shouldn't push (no metadata yet); they just wait for OpenSF to provide

## Verification

Edit a field in app, confirm push to OpenSFHistory within seconds (check via website or API). Edit a field on OpenSFHistory, wait for next pull, confirm appears in app. Force a concurrent edit collision, confirm conflict dialog renders. Hard to verify without OpenSFHistory access — likely needs staging environment.

## Estimated size

**L.** Risk: significantly larger if the API surface is uncooperative (no delta endpoint, etc.) — re-scope on first contact with API docs.
