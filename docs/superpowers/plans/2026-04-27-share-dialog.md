# Plan 5 — Ad-hoc share dialog

> **Status:** Active. Decisions locked 2026-05-01.
>
> **Position in roadmap:** Plan 5 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 2 (Detail view's header bar with Share button). Reuses the B2/S3 client + resize pipeline from Plan 4 (`sharing.rs::fulfill_order`).

## Goal

A polished one-off image share flow: in Detail view, click Share (or `⌘⇧S`) → dialog with recipient email, resolution preset, purpose textarea → backend resizes the image → uploads to B2 → POSTs to OpenSFHistory which sends the email via Postmark. Audit log captures every share.

## Resolved decisions

| Question | Decision |
| --- | --- |
| Email mechanism | **OpenSFHistory API as proxy.** Desktop app POSTs the share metadata + signed URL to a new OpenSFHistory endpoint (`POST /api/share-links`). OpenSFHistory uses its existing Postmark integration to send the email with a templated body. No SMTP credentials in the desktop app; no direct Postmark coupling. |
| Resolution presets | **Low 800px / High 2048px / Full (original).** Same resize pipeline as `fulfill_order` (Lanczos3, JPEG quality 90). Sizes already live in app_settings (`resolution_low_px`, `resolution_high_px`); Full means no resize. |
| Link expiry | **B2 public URL + lifecycle policy (30d).** URLs are permanent within the bucket until B2's lifecycle rule (configured on the B2 side) deletes objects in `shares/` older than 30 days. Matches Plan 4's `s3_public_base_url` model — no per-link revocation in the desktop app. |

## Scope

**Backend (`src-tauri/src/sharing.rs`):**

- New `create_share_link` command:
  - Args: `image_id`, `recipient_email`, `resolution` (`"low"|"high"|"full"`), `purpose`
  - Reads same settings as `fulfill_order` (Laravel API URL/token, B2 endpoint/bucket/keys, resolution presets) + new `s3_share_prefix` (default `"shares"`)
  - Resolves `file_path` + `catalog_number` + `title` from image_id
  - Resizes image to a temp file (or skips if "full")
  - Generates a random 16-hex share key (via `argon2::password_hash::rand_core::OsRng` — already a dep)
  - Uploads to B2 at `<prefix>/<catalog>-<random>.jpg`
  - Builds public URL = `<s3_public_base_url>/<prefix>/<key>`
  - POSTs to OpenSFHistory `/api/share-links` with the payload
  - Inserts `usage_log` entry (same shape as `fulfill_order`'s entries)
  - Inserts `audit_log` entry tagging the share (field_name = `"shared"`, `new_value` = recipient email)
  - Returns `{ image_url, recipient_email, resolution_label }` for the success toast

**Frontend:**

- New `src/lib/commands/sharing.ts`: `createShareLink(...)` wrapper
- New `src/lib/components/sharing/ShareDialog.svelte`:
  - Image preview (small thumb, catalog number, title)
  - Recipient email input (validated email format, required)
  - Resolution radio group (Low / High / Full) — default High
  - Purpose textarea (4 rows)
  - Usage-rights callout: shown when `image.usage_rights` is non-empty
  - Cancel + "Resize & send" buttons
  - Inline error/success state
- Wire to `DetailView.svelte`:
  - Existing Share button (header bar) opens the dialog
  - `⌘⇧S` shortcut while DetailView is mounted

## Out of scope

- Multi-image share. Detail view is single-image; the dialog shares exactly one.
- Custom expiration per-share. Always 30d-via-B2-lifecycle.
- "Shares history" view. The audit log already captures every share; if a dedicated view is wanted later, it's a separate plan.
- Postmark template editing inside the desktop app. Templates live on the OpenSFHistory side.
- Direct SMTP / direct Postmark from desktop. We always proxy through OpenSFHistory.

## OpenSFHistory side (not implemented here)

The desktop app assumes `POST /api/share-links` exists. Expected contract:

**Request body:**

```json
{
  "catalog_number": "wnp27.4283",
  "title": "Sutro Baths, exterior",
  "recipient_email": "alice@example.com",
  "purpose": "Used in our newsletter",
  "image_url": "https://b2.example.com/shares/wnp27.4283-a1b2c3d4.jpg",
  "resolution_label": "High (2048px)",
  "sender_username": "daniel",
  "expires_at": "2026-06-01T12:34:56Z"
}
```

`expires_at` is ISO 8601 UTC. Defaults to 30 days from send; configurable
via the `share_link_expires_days` app setting so it can be kept in sync
with the B2 lifecycle rule on the `shares/` prefix. The email template
on the OpenSFHistory side can render it however it likes.

**Response:** `200 OK` on success; non-2xx returns `{ error: "..." }`.

**OpenSFHistory's responsibility:**

- Authenticate via Bearer token (same as orders API)
- Compose a Postmark email with a templated body
- Send via the existing Postmark integration
- Optionally log the share server-side

**Until this endpoint exists,** the desktop command will return an error from the POST call.

## Risks + considerations

1. **OpenSFHistory endpoint doesn't exist yet.** The command will fail until it's wired up. The dialog surfaces the error; user implements the endpoint, retries.
2. **B2 lifecycle policy.** 30-day deletion needs to be configured on the B2 bucket via the B2 console (one-time).
3. **Failure mid-flow.** If B2 succeeds but the OpenSFHistory POST fails, the image sits in B2 with no email sent. Lifecycle eventually cleans up. Acceptable for v1.
4. **Multiple shares of the same image.** Each share gets a different random key + URL — no dedup. Cost is small.

## Estimated size

**S–M.** Roughly 3–5h. Backend ~1.5h (variant of fulfill_order), Frontend ~1.5h (one dialog), wire-up + smoke ~1h.
