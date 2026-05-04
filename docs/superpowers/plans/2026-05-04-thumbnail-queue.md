# Plan 13 — Background thumbnail queue + footer activity indicator

> **Status:** Proposed.
>
> **Position:** Builds on Plan 12. Decouples thumbnail generation from
> the scan/import path so the library is navigable immediately after
> a scan completes.

## Goal

Today, every code path that adds images to the index (initial setup,
re-scan, watcher events) blocks while it runs `extract_exif_thumbnails_batch`
over every pending image. For 50K-image archives that's a ~10–30 minute
freeze. Permanent failures (corrupt files, formats `image-rs` can't decode
even with limits off) get retried on every scan with no record.

After this plan: thumbnails generate in a background worker, scans
return immediately, the library renders placeholders for missing
thumbnails (already supported), and a footer indicator surfaces
progress + failures.

## Architecture

### Persistent state

- New column `images.thumbnail_state` — `pending` | `done` | `failed`.
- Default `pending`. Set to `done` after a successful generate. Set to
  `failed` (with `thumbnail_error` text) when `image-rs` returns an
  error we can't recover from.
- Migration 005: add the columns. Backfill existing rows: if
  `thumbnail_path IS NOT NULL` → `done`, else → `pending`.

### Worker

- New module `thumbnail_queue.rs`. Spawns one worker thread at app boot
  (lifecycle plumbing matches `drive::spawn_drive_poller`).
- Loop:
  1. `SELECT id, file_path FROM images WHERE thumbnail_state = 'pending' LIMIT 32`
  2. For each: try EXIF extract → fall back to full decode → mark `done` /
     `failed` accordingly.
  3. Emit `thumbnails:progress { totalPending, processed, failed }` after
     each batch.
  4. If batch was empty, sleep 5s then loop. If non-empty, loop immediately.
- Single thread for v1. The archive drive is usually the bottleneck;
  parallelism wins less than you'd expect and complicates lifecycle.

### Triggers

- `scan_directory` no longer runs the thumbnail batch inline. New rows
  land with `thumbnail_state = 'pending'` and the worker picks them up.
- The Sidebar's `library:filesystem-changed` listener stops calling
  `extractExifThumbnailsBatch`. Just `scanDirectory` + `refreshSourceTree`.
- Settings → General "Re-scan" button: same — drops the thumbnail
  status string (the footer indicator covers it now).
- Setup flow: `scan` → `metadata` → straight to library. No
  thumbnail-progress page.

### Frontend

- New `thumbnailProgress` store fed by the Tauri event.
- New footer component placed next to `DriveIndicator`. States:
  - **Idle** (`totalPending === 0 && failed === 0`): hidden, or a small
    green check.
  - **Active**: spinner + "Generating thumbnails: 1,247 / 50,000".
  - **Failed**: pill turns destructive — "5 failed", click opens a
    popover with the first ~10 error rows + a "Retry failed" button.
- New command `retry_failed_thumbnails` that sets `failed → pending` for
  every image (or by id list).

## Phases (commits)

1. **Backend** — migration 005, `thumbnail_queue.rs` worker, AppState
   plumbing, `retry_failed_thumbnails` command. Existing
   `extract_exif_thumbnails_batch` becomes a thin "kick the worker" call
   (kept for backwards-compat with the import flow's current shape).
2. **Setup unblock** — ImportProgress drops the thumbnails stage; routes
   to library after metadata. Sidebar + Settings stop calling the
   thumbnail batch.
3. **Footer indicator** — `thumbnailProgress` store, footer component,
   popover with retry.
4. **Cleanup** — drop now-unused code paths, update CLAUDE.md notes.

## Out of scope

- Multi-worker / GPU acceleration. Punt unless single-thread is
  noticeably slow.
- Per-image cancellation. Retry-failed is the only direct control.
- Re-generation (e.g. user changes thumbnail size). Could be a future
  command that flips all rows back to `pending`.
- Smart prioritization (e.g. "thumbnails for currently-viewed source
  first"). The grid already lazy-loads via `generate_thumbnail_single`
  for visible images, so the placeholder→thumb fade-in is fine.

## Open questions

1. **Initial setup metadata stage** — keep in the foreground (one
   exiftool pass over the source dir, typically ~30s for 50K files), or
   move to background too? My instinct is keep it foreground since it's
   one-shot and the user expects setup to take a beat. Confirm?

2. **Two-tier thumbnails** — current code does EXIF-extract → full-decode
   fallback. Worker keeps both? Or simplify to full-decode only? EXIF
   is genuinely faster (~10ms vs ~500ms per image), but the two paths
   add code. Lean toward keeping.

3. **Failure surfacing** — for a v1, is "5 failed thumbnails [Retry
   failed]" enough, or do you want a list of failed files / their error
   messages? List is more useful but more UI work.

4. **Worker idle interval** — 5s feels right (latency tolerable, idle
   CPU near zero). Anything you'd prefer? Could also wake-via-channel
   when scan/watcher enqueues, but the polling fallback is simple and
   has bounded latency.
