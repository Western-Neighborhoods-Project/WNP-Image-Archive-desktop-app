# Security notes

This app runs on two office iMacs connected to a Synology NAS. It is a
single-purpose internal tool used by trusted staff, and the notes below
describe the threat model it's built for and the operational steps that keep
it safe.

## Where credentials live

The app stores three secrets so it can talk to external services:

- **S3 access key** and **S3 secret key** — for uploading resized images to the
  B2/S3 bucket.
- **Laravel API token** — for the OpenSFHistory image-request API.

These are kept in the app's local SQLite database
(`~/Library/Application Support/org.wnp.imagearchive/archive_manager.db`), in
the `app_settings` table. They are **not encrypted at rest**. Over the IPC
boundary they are gated: the frontend can only read them through the admin-only
`get_setting` command, never through `get_public_setting`.

### What protects them

- **macOS user isolation.** `~/Library` is not readable by other local user
  accounts, so a second user on the same iMac cannot read the database. The app
  additionally sets the app-data directory to `0700` and the database file to
  `0600` on startup as defense in depth (`src-tauri/src/db.rs`).
- **Admin-only IPC read.** Only an authenticated admin session can read the
  secret settings back out through the app.

### What does NOT protect them, and what to do about it

Plaintext-at-rest means anyone who can read the **raw disk or a backup** can
read the credentials. To cover that:

1. **Enable FileVault** (full-disk encryption) on both iMacs. This is the single
   most important step — it protects the credentials (and everything else) if a
   machine or drive is lost, stolen, or decommissioned. System Settings →
   Privacy & Security → FileVault.
2. **Don't back up the app-data directory unencrypted.** If Time Machine or any
   NAS backup captures `~/Library/Application Support/org.wnp.imagearchive/`,
   ensure that backup target is itself encrypted.
3. **Use separate macOS accounts** (not one shared login) if you want per-user
   isolation on a shared machine; otherwise every operator shares the same
   local data.
4. **Rotate the keys** if a machine is ever lost or disposed of without having
   had FileVault enabled.

A future hardening option is to move the three secrets into the macOS Keychain.
That was deferred deliberately: the app is currently unsigned and auto-updates
frequently, and macOS scopes Keychain access to an app's code signature, so
without a stable Apple Developer ID an update can revoke Keychain access and
force credentials to be re-entered. Keychain migration pairs best with proper
code-signing and should be revisited then.

## Authentication

- Passwords are hashed with **Argon2id** (`src-tauri/src/auth.rs`).
- Login is **rate-limited**: after 5 failed attempts a username is locked out
  for 60 seconds, then the counter resets to a fresh (re-lockable) budget.
- Changing your own password requires the **current password**; admins can reset
  another user's password without it.
- Sessions live in memory and are dropped when the app closes. An **inactivity
  timeout** logs the user out after a configurable idle period.

## Reporting

This is an internal tool. Report any security concern directly to the
maintainer.
