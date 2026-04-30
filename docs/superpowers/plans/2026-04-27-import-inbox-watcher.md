# Plan 7 — Import inbox watcher

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 7 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (primitives). Plan 6 (drive monitoring — watcher needs to know when drive is mounted to start/pause).

## Goal

Replace one-shot bulk import with continuous file-system watching of `/Volumes/WNP-Archive/_inbox/`. New images dropped onto the drive (outside the app) auto-import into per-batch groups awaiting metadata.

## Scope

- Backend file watcher using `notify` crate, scoped to `<drive>/_inbox/`
- Batch detection heuristic (likely: time-gap based — if no new file added in 5 min, close batch; new file after that opens new batch)
- New `import_batches` table (id, label, source, created_at, total, synced_count) — schema migration needed
- Auto-import-on-drop: extract EXIF, generate thumbnail, insert image with `awaiting_metadata = true` flag
- Existing setup-time bulk import becomes "first run" only; thereafter the inbox watcher takes over
- New `ImportInboxView.svelte`: batch list (left, 320px) + batch detail (right) with thumbnails + per-image sync status chip (Synced / Awaiting)
- "Open in OpenSFHistory ↗" link per batch (deep-link to website's metadata editor)

## Out of scope

- The actual OpenSFHistory metadata sync (Plan 9 — this plan only marks images "awaiting" and links out)
- File deletion handling (deletes outside the app are out of scope)

## Key files

**New:**
- `src-tauri/src/watcher.rs` (notify-based watcher, debounced events)
- `src-tauri/src/import_inbox.rs` (batch logic, auto-import)
- `src/lib/components/inbox/ImportInboxView.svelte`, `BatchList.svelte`, `BatchDetail.svelte`, `BatchImageCard.svelte`

**Modify:**
- `src-tauri/src/db.rs` (new `import_batches` table + schema migration)
- `src-tauri/src/scanner.rs` (extract reusable single-file import path so the watcher can call it)
- `src-tauri/src/lib.rs` (register watcher, wire to drive events)
- `src/routes/+page.svelte` (replace `ImportInboxStub`)
- `src/lib/components/setup/SetupScreen.svelte` (clarify: setup = first-time bulk; ongoing = inbox)

## Open questions

- Batch boundary heuristic — time-gap, folder name, or manual marker file?
- What happens to images added during the initial bulk import — same batch as initial or separate?
- Schema migration strategy for existing users (production DB has rows)
- Watcher behavior when drive is disconnected (idle? error? auto-resume on reconnect — likely via Plan 6 events)
- Concurrency: how many simultaneous file imports

## Verification

With watcher running, drop a TIFF into `_inbox/`, confirm it appears in the inbox view within seconds. Drop several over a few minutes, confirm batch grouping. Disconnect/reconnect drive — confirm watcher pauses/resumes cleanly. Visual diff against artboard 6.

## Estimated size

**L.**
