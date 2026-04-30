# Plan 8 — Backblaze B2 backups

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 8 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (primitives). Plan 6 (drive monitoring — backups need drive mounted).

## Goal

Whole-archive sync to Backblaze B2 with a monitoring UI matching design artboard 7.

## Scope

- Backend: new `backups.rs` module — periodic scan of catalog vs B2 manifest, upload queue with retry/exponential-backoff, schedule (nightly incremental + weekly full verify), status events
- New `backup_log` table (file, action, started_at, completed_at, status, error)
- New `BackupsView.svelte` with five tabs: **Overview** (the dashboard with stats / 30-day chart / what's-backed-up / live-upload / failures), **Queue**, **Failures**, **Restore**, **Schedule**
- Settings page for B2 config (key, bucket, endpoint, region) — adds the "Backup" sub-page in Settings (Plan 2 scaffolds the stub)
- Dashboard data: counts, sizes, time-series

## Out of scope

- Actual restore flow (stub the Restore tab — restore is rare and needs careful UX)
- Cross-region replication
- Manual file-level browse of the B2 bucket

## Key files

**New:**
- `src-tauri/src/backups.rs` (sync engine), `backup_schedule.rs` (cron-like scheduler)
- `src/lib/components/backups/BackupsView.svelte`, `OverviewTab.svelte`, `QueueTab.svelte`, `FailuresTab.svelte`, `RestoreTab.svelte`, `ScheduleTab.svelte`, `BackupActivityChart.svelte`

**Modify:**
- `src-tauri/src/db.rs` (new `backup_log` table)
- `src-tauri/src/sharing.rs` — extract a shared `b2_client.rs` so backups + share-link uploads use one S3 client
- `src/routes/+page.svelte` (replace `BackupsStub`)
- Settings sub-page "Backup" (Plan 2 created stub) with B2 credentials form

## Open questions

- What gets backed up: originals only, originals + thumbnails, originals + thumbnails + DB snapshots + audit log? (Design implies all four.)
- Concurrency limit for uploads (default 4? 8?)
- Drive-disconnected case — backup paused vs error
- Encryption at rest (B2 native + client-side?)
- How to handle manifest drift (e.g., file moved on drive but B2 still has old path)

## Verification

Configure B2 in settings, kick off a backup, monitor live progress. Force a failure (e.g., bad checksum), confirm failure entry appears. Run weekly verify, confirm completes. Visual diff against artboard 7.

## Estimated size

**L.**
