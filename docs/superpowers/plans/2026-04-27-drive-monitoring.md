# Plan 6 — Drive monitoring

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 6 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (`StatusBar` primitive). Unblocks Plans 7 and 8.

## Goal

Add the persistent drive indicator + hover popover + disconnected nag screen. App needs to know when `/Volumes/WNP-Archive` is mounted because image rendering, exports, shares, and backups all depend on it.

## Scope

- Backend: detect mount status of `/Volumes/WNP-Archive` (configurable path), poll free-space every 30s, emit Tauri events on mount/unmount transitions
- New `driveStatus` Svelte store fed by Tauri events
- New `DriveIndicator.svelte` rendered inside every view's `StatusBar` slot — green dot + label + size when connected, red dot + "Disconnected" when not
- New `DriveIndicatorPopover.svelte` shown on indicator hover/click — full stats (used / total, image count, format mix, last scan, mounted duration), Reveal-in-Finder button, Backup-status link
- New `DriveDisconnectedScreen.svelte` shown as overlay in main content area when drive is offline — "Retry connection" + "Settings" buttons + last-seen / last-backup info
- Read-only mode when disconnected: metadata views work, but image rendering / export / share / fulfill all show clear blocked state

## Out of scope

- Multi-drive support (assumes one canonical drive)
- Custom drive paths beyond a single configurable canonical drive

## Key files

**New:**
- `src-tauri/src/drive.rs` (mount detection, polling, event emission)
- `src/lib/stores/driveStatus.ts` (subscribes to Tauri events)
- `src/lib/components/drive/DriveIndicator.svelte`, `DriveIndicatorPopover.svelte`, `DriveDisconnectedScreen.svelte`

**Modify:**
- `src-tauri/src/lib.rs` (register, set up tauri::async_runtime poller)
- Every view's StatusBar usage to render `DriveIndicator`
- `src/routes/+page.svelte` (drive-disconnected overlay logic)
- Settings (add "Archive drive" config — drive name + expected path)

## Open questions

- Default drive name "WNP-Archive" hard-coded vs settings field?
- Polling interval (15s? 30s? 60s?)
- What read-only ops should be blocked vs warned (probably block: render full image, export, share, fulfill; allow: read metadata, browse thumbnails from cache)
- Behavior on app startup with drive already missing — show nag immediately or attempt one mount probe first

## Verification

With drive mounted, hover indicator → confirm popover renders with correct stats. Eject drive → indicator turns red within polling interval, nag screen overlays main content. Reconnect → state recovers. Visual diff against artboards 5a / 5b.

## Estimated size

**M.**
