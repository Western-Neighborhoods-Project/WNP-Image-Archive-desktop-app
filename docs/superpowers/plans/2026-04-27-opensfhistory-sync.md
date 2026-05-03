# Plan 9 — OpenSFHistory metadata sync

> **Status:** Active. Decisions locked 2026-05-01. Scoped down dramatically from the original spec — this is a read-only one-way sync from the OpenSFHistory API into the desktop catalog. Future Phase 2 will wire the push-back direction.
>
> **Position in roadmap:** Plan 9 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 2 (DetailView). Reuses the `build_authed_client` helper + `laravel_api_url` + `laravel_api_token` settings from `sharing.rs`.

## Goal

When the user opens an image's detail view, fetch `GET /photo/{catalog_number}` and treat that response as the source of truth for metadata. API-mapped fields populate the existing detail view inputs and become read-only (until Phase 2 wires push-back). Local-only fields (internal_notes, donor, etc.) stay editable.

## Resolved decisions

| Question | Decision |
| --- | --- |
| Edit lock | **API-synced fields are read-only for now.** Locked: title, description, city/state/country, date_display, date_start, date_end, photographer, usage_rights, plus all new mirror columns. Editable: internal_notes, donor, acquisition_date, keywords, archival_collection. The lock is lifted in a future plan when push-back is wired. |
| Fetch trigger | **5-minute TTL with local cache.** First detail view → fetch + write to local DB + set `last_synced_at`. Subsequent views within 5 min skip the fetch (local is "fresh"). Manual "Re-sync" button is available regardless. |
| Unmapped API fields | **Extend the schema.** New columns on `images`: `caption`, `dimensions`, `format`, `publisher`, `citation`, `download_permitted`, `neighborhoods` (JSON array), `photosets` (JSON object), `osf_collections` (JSON array), `osf_page_url`, `last_synced_at`. |

## API → local field mapping

| API field | Local column | Notes |
| --- | --- | --- |
| `catalog_number` | `catalog_number` | direct (already exists) |
| `title` | `title` | direct |
| `caption` | `caption` (new) | direct |
| `description` | `description` | direct |
| `date_taken` | `date_display` | direct (human-readable) |
| `year` | `date_start` | converted: `"1924"` → `"1924-01-01"`. `date_end` left alone. |
| `location` | `city`, `state`, `country` | parsed: comma-split. `"San Francisco, CA, USA"` → `city="San Francisco"`, `state="CA"`, `country="USA"`. Single-part lands in `city`. |
| `dimensions` | `dimensions` (new) | direct |
| `format` | `format` (new) | direct (e.g. `"JPEG"`, `"TIFF"`) — also retroactively fixes the latent `format_mix` query in `drive.rs` |
| `contributor` | `photographer` | direct |
| `publisher` | `publisher` (new) | direct |
| `citation` | `citation` (new) | direct |
| `copyright` | `usage_rights` | direct |
| `download_permitted` | `download_permitted` (new) | bool → INTEGER 0/1 |
| `page_url` | `osf_page_url` (new) | direct |
| `neighborhoods` | `neighborhoods` (new) | JSON array as TEXT |
| `photosets` | `photosets` (new) | JSON object as TEXT |
| `collections` | `osf_collections` (new) | JSON array as TEXT (distinct from local `collections` table) |
| `display_url`, `file_path`, `date_added` | — | not stored locally; we already have local file_path + created_at |

## Scope

**Backend:**

- New columns on `images` (Migration 003 in `db.rs`):
  - `caption TEXT`
  - `dimensions TEXT`
  - `format TEXT`
  - `publisher TEXT`
  - `citation TEXT`
  - `download_permitted INTEGER`
  - `neighborhoods TEXT` (JSON array)
  - `photosets TEXT` (JSON object)
  - `osf_collections TEXT` (JSON array)
  - `osf_page_url TEXT`
  - `last_synced_at TEXT`
- Update `models::ImageRecord` to include the new fields.
- Update `queries::row_to_image_record` and the SELECT in `query_images` / `get_image` to surface them.
- New module `opensf_sync.rs`:
  - `OpenSfPhotoResponse` struct mirroring the API shape
  - `sync_image_from_opensf(image_id, force)` command:
    - Reads `laravel_api_url`, `laravel_api_token`, image's catalog_number from DB
    - Checks `last_synced_at`; if within 5 min and `!force`, returns the existing record
    - Otherwise GET `/photo/{catalog_number}`
    - Maps fields → writes to DB → bumps `last_synced_at`
    - Returns the updated `ImageRecord`
  - Graceful degradation: API errors don't crash the view; the command logs and returns the existing record
- Update `editor::EDITABLE_FIELDS` to **remove** the now-locked fields (or keep them but the UI gates the inputs — see Frontend below). For belt-and-suspenders we'll do both.

**Frontend:**

- `src/lib/commands/opensfSync.ts` — wrapper for the new command
- `DetailView.svelte`:
  - `onMount`: call `syncImageFromOpensf(imageId, force=false)`. If it returns a fresh record, swap the displayed image to the new one.
  - Each synced field's input gets a `disabled` attribute (read-only) plus a small lock icon (or muted styling) so the user understands why.
  - Add display rows for the new fields (caption, dimensions, format, publisher, citation, neighborhoods chips, photosets chips, osf_collections chips, "View on OpenSFHistory" link)
  - Small "Last synced 4m ago" indicator + manual "Re-sync" button in the inspector header

## Out of scope

- Push-back from desktop edits to OpenSFHistory. Lifted in a future plan.
- Conflict resolution (local edit vs remote change). Doesn't apply yet because edits are locked.
- Sync status indicator in the StatusBar across all views. Detail-view-only for v1.
- Periodic background polling. We re-fetch only on detail view open.
- Bulk re-sync of the whole catalog (50K images would hammer the API). Per-image only.

## Risks + considerations

1. **API endpoint not yet exposed in dev/stage.** The user already confirmed it returns the documented shape. If the staging instance ever returns a different schema, the deserialization fails and the command logs an error — we fall back to local data. No detail-view crash.
2. **API down / network offline.** Same fallback: log + use local. The user can still browse what's already in the DB.
3. **Schema drift** if the API adds fields. New fields appear in the JSON but `serde` ignores unknown ones by default — safe.
4. **Drive monitor's `format_mix` query** silently fails today because `format` doesn't exist. Adding the column here fixes that retroactively (no separate fix needed).
5. **Read-only banner / lock UX.** Need to communicate clearly that edits aren't allowed yet. A small lock icon next to disabled fields + a one-line note at the top of the inspector should be enough.
6. **First-time sync of an existing 50K-image DB.** No bulk migration — `last_synced_at` is null for all rows; the next time the user opens any image, it'll sync. Over time the cache fills naturally.

## Estimated size

**M.** Roughly 5–7h. Schema + migration ~1h, backend module ~2h, ImageRecord + queries plumbing ~1h, frontend lock + display ~2h, verify ~1h.
