# Plan 5 — Ad-hoc share dialog

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 5 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 2 (refactored Detail view with Share button wired in).

## Goal

Implement the design's polished `ShareDialog` — staff sends a one-off image link to an external recipient. Different from order fulfillment (Phase 4) in that it's user-initiated, not in response to an OpenSFHistory order.

## Scope

- New `ShareDialog.svelte`: image preview row, recipient email input, three-option resolution picker (Low/High/Full), purpose textarea, usage-rights warning callout, Cancel / "Resize & send" buttons
- Backend command: `create_share_link(image_id, email, resolution, purpose)` — resize image to target → upload to Backblaze B2 → return signed URL → email recipient (or return URL for Tauri to open mail composer)
- Audit log entry on share
- Triggered from Detail view's Share button (⌘⇧S)
- Link expiry default 30 days (per design copy)

## Out of scope

- Link revocation
- Recipient analytics
- Multi-image share (one image at a time; multi-image is what fulfillment is for)

## Key files

**New:**
- `src/lib/components/sharing/ShareDialog.svelte`

**Modify:**
- `src-tauri/src/sharing.rs` (extend with `create_share_link` — module already handles fulfill_order S3 upload; this reuses much of it)
- `src-tauri/src/lib.rs` (register new command)
- `src/lib/components/detail/DetailView.svelte` (wire Share button — note: Detail view from Plan 2)

## Open questions

- Email mechanism: SMTP from the desktop app, send via OpenSFHistory API as proxy, or just open the user's mail client with mailto link?
- Resolution preset dimensions (Low ~800px, High ~2048px, Full = original — per design)
- Link expiry: B2 signed URL with TTL, or permanent + revoke-list?
- Reuse existing fulfill_order resize logic, or extract a shared resize utility?

## Verification

From Detail view, click Share, fill dialog, submit. Confirm audit entry appears. Open recipient inbox / mail client to confirm link arrives. Visual diff against artboard 4c.

## Estimated size

**S–M.**
