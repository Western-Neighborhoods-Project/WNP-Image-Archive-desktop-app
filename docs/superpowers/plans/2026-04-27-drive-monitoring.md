# Plan 6 — Drive monitoring

> **Status:** Active. Decisions locked 2026-05-01.
>
> **Position in roadmap:** Plan 6 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (`StatusBar` primitive). Unblocks Plans 7 and 8.

## Goal

Persistent drive indicator + popover stats + hard-block disconnected screen. App needs to know when the archive volume is mounted because image rendering, exports, shares, and backups all depend on it.

## Resolved decisions

| Question | Decision |
| --- | --- |
| Drive name + path source | **Reuse existing `source_directory` setting.** Drive root is derived (the volume that contains it). No new settings field. |
| Polling interval | **15s** for stats (free-space, image count, format mix). Mount-state probe runs every **1s** (essentially free — single `Path::exists` call). |
| Disconnect blocking | **Hard block.** No dismiss. Disconnect overlay covers the main content area. Sidebar stays navigable so user can reach Settings; everything else (Library/Detail/Requests/Audit/Recently/Inbox/Backups/SmartCollections) shows the overlay. |
| Startup with drive missing | App boots normally. Initial 1s mount probe fires immediately on backend startup; if disconnected, overlay renders as soon as the store hydrates — no flash of usable UI. |

## Scope

- Rust: `drive.rs` module with mount detection (1s `Path::exists` probe), free-space + image-count polling (15s), Tauri events on transitions
- `driveStatus` Svelte store fed by Tauri events
- `DriveIndicator.svelte` rendered inside every view's `StatusBar` slot
- `DriveIndicatorPopover.svelte` shown on indicator click — full stats + Reveal-in-Finder
- `DriveDisconnectedScreen.svelte` covers main content area when disconnected — Retry + go-to-Settings buttons
- Read live drive state in General settings page (next to existing source-directory display)

## Out of scope

- Multi-drive support
- Auto-recovery beyond manual Retry button (user can hit Retry; or just fix the drive — 1s probe will pick it up)
- The "last backup at …" line on the disconnect screen — Plan 8 is the source of truth here. We render "Backups not configured" until Plan 8 lands.
- Behavior changes to existing destructive "Change source directory…" button — unchanged

## Key files

**New (Rust):**
- `src-tauri/src/drive.rs`

**New (Svelte):**
- `src/lib/stores/driveStatus.ts`
- `src/lib/components/drive/DriveIndicator.svelte`
- `src/lib/components/drive/DriveIndicatorPopover.svelte`
- `src/lib/components/drive/DriveDisconnectedScreen.svelte`

**Modify:**
- `src-tauri/Cargo.toml` (add `fs4`)
- `src-tauri/src/lib.rs` (register commands + spawn background poller)
- `src/routes/+page.svelte` (init listener, render disconnect overlay)
- `src/lib/components/browsing/LibraryView.svelte`, `RecentlyViewedView.svelte`, `audit/AuditLogView.svelte`, `requests/RequestsView.svelte`, `stubs/StubView.svelte` — render `DriveIndicator` inside their StatusBar
- `src/lib/components/settings/pages/GeneralPage.svelte` — show live drive state

## Architecture

```
                 source_directory (existing setting)
                            │
                            ▼
                  ┌────────────────────┐
                  │   drive.rs (Rust)  │
                  │ ─ 1s mount probe   │
                  │ ─ 15s stats poll   │
                  │ ─ emits events:    │
                  │   drive:mounted    │
                  │   drive:unmounted  │
                  │   drive:stats-upd  │
                  └─────────┬──────────┘
                            │ Tauri events
                            ▼
                ┌──────────────────────┐
                │ driveStatus store    │
                │ (Svelte writable)    │
                └─────────┬────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
  DriveIndicator   DriveDisconn-       GeneralPage
  + Popover        Screen overlay      live status
  (in StatusBar)   (in +page.svelte)   line
```

## Verification

- App boots with drive mounted → green indicator, popover shows stats (used / total, image count, format mix)
- Eject drive → within ~1s indicator + overlay flip to disconnected
- Reconnect → state recovers, no app restart needed
- Settings reachable from disconnect overlay via sidebar; existing Change-source-directory still works
- `bun run check` clean; `cargo check` clean

## Estimated size

**M.**
