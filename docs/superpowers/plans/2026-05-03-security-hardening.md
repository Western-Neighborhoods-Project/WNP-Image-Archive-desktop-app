# Plan 11 — Security hardening

> **Status:** Proposed. Findings from 2026-05-03 security review.
>
> **Position in roadmap:** Cross-cutting; runs in parallel with feature work. Phases are independently shippable.
>
> **Depends on:** Plan 10 (auth + user management — already merged). Several tasks below add backend role gates that assume `auth::require_admin` / `current_session` exist.

## Goal

Close the security gaps identified in the 2026-05-03 codebase review. The current build relies on the frontend to enforce most invariants (role gating, secret-key visibility, input validation); this plan moves those checks into the Rust backend so the WebView is no longer a trust boundary.

The work is grouped into four phases, ordered by exploitability. **Phases 1 and 2 should land before any external testing.** Phases 3 and 4 are hardening + tech-debt and can be sequenced into normal feature work.

## Threat model assumptions

- **Single-user macOS desktop app.** No multi-tenant boundary; the threat is not other authenticated users abusing the system.
- **WebView XSS is the primary concern.** Image titles, captions, and descriptions are populated from external sources (OpenSFHistory API, ExifTool reading user-supplied image files). Any XSS sink → full Tauri command execution → credential exfiltration → bucket compromise.
- **Editor accounts are not fully trusted.** A compromised or malicious editor must not be able to read S3 secrets, reset the catalog, or escalate to admin.
- **The local SQLite DB is readable by any process running as the user.** Anything stored unencrypted there is one shell command away from any unprivileged software on the machine.

## Resolved decisions

| Question | Decision |
| --- | --- |
| Where do secrets live long-term? | ~~macOS Keychain via the `keyring` crate.~~ **Reverted 2026-05-04** — secrets stay in `app_settings`. See "Keychain reversal" below. |
| What's the auth gate granularity? | Two helpers: `require_session` (any logged-in user) and `require_admin` (admin only). Existing `require_admin` in `auth.rs` stays; add `require_session`. Every state-mutating command calls one of them. |
| Read-only commands? | `get_setting` is split: a public `get_public_setting(key)` for whitelisted UI keys, and the existing `get_setting` becomes admin-only for secret keys. |
| Backwards compatibility for existing installs? | ~~One-shot migration on app start: read plaintext secrets from `app_settings`, write to Keychain, DELETE from `app_settings`.~~ **No-op** after the Keychain reversal. |
| New tests? | Unit tests for `csv_escape`, `validate_uuid`, `sanitize_catalog_number`, and the auth helpers. Integration tests are out of scope for this plan. |

### Keychain reversal (2026-05-04)

After the initial Phase 3 implementation we discovered that the `keyring` crate's macOS backend is unreliable for unsigned dev builds: writes appeared to succeed but reads came back empty, because each `cargo build` produces a binary with a different effective code-signing identity and the per-item ACL excludes the next build's identity.

The threat model decision: this is a single-user macOS desktop app. If an attacker has shell access as the user, they already have bigger problems than reading these credentials. Both OpenSFHistory and the B2 bucket are under our control, so key rotation is trivial. The marginal security benefit of Keychain didn't justify the dev-mode friction.

So we reverted: secrets live in `app_settings` alongside everything else. The `is_secret` allowlist still exists in `settings.rs` so `get_public_setting` rejects credential reads (defence-in-depth against an editor account or compromised renderer doing `invoke('get_public_setting', { key: 's3_secret_key' })` from devtools), and `get_setting` for those keys still requires admin role. What changed: no Keychain hop on read or write, and the `keyring` crate is no longer a dependency.

If your install was upgraded to an intermediate Plan 11 build that wrote secrets into Keychain, those entries are orphaned. Re-enter credentials in the API settings page and (optionally) clean up with `security delete-generic-password -s org.wnp.imagearchive -a s3_secret_key` (same pattern for `s3_access_key`, `laravel_api_token`).

---

## Phase 1 — Immediate fixes (target: one PR, ship before next external interaction)

These are <100 lines total, no schema changes, no migrations. They close the four most exploitable holes.

### Task 1.1 — Add `require_session` helper

**File:** `src-tauri/src/auth.rs`

After the existing `require_admin` (line 248), add:

```rust
/// Helper used to gate commands that need any logged-in user.
/// Use over `require_admin` when an editor is also allowed (e.g. metadata
/// editing, share creation, order fulfillment).
pub fn require_session(state: &State<AppState>) -> Result<UserSession, String> {
    current_session(state).ok_or_else(|| "Not logged in".to_string())
}
```

**Acceptance:** `cargo build` clean. No callers yet — that's Task 1.2+.

---

### Task 1.2 — Gate every state-mutating command on a session check

Add `auth::require_session(&state)?;` (or `auth::require_admin(&state)?;` per the table below) as the **first line** of each command listed.

| File | Function | Line | Gate |
| --- | --- | --- | --- |
| `src-tauri/src/settings.rs` | `set_setting` | 59 | `require_admin` |
| `src-tauri/src/settings.rs` | `reset_catalog` | 80 | `require_admin` |
| `src-tauri/src/scanner.rs` | `scan_directory` | 40 | `require_admin` |
| `src-tauri/src/sharing.rs` | `fetch_orders` | 101 | `require_session` |
| `src-tauri/src/sharing.rs` | `fulfill_order` | 130 | `require_session` |
| `src-tauri/src/sharing.rs` | `fail_order` | 306 | `require_session` |
| `src-tauri/src/sharing.rs` | `create_share_link` | 357 | `require_session` |
| `src-tauri/src/editor.rs` | `update_image_metadata` | 41 | `require_session` |
| `src-tauri/src/editor.rs` | `write_metadata_to_file` | 314 | `require_session` |
| `src-tauri/src/editor.rs` | `export_audit_log_csv` | 222 | `require_session` |
| `src-tauri/src/editor.rs` | `log_image_view` | 433 | `require_session` |
| `src-tauri/src/metadata.rs` | `extract_metadata_batch` | 110 | `require_admin` |
| `src-tauri/src/metadata.rs` | `extract_metadata_single` | 220 | `require_admin` |
| `src-tauri/src/thumbnails.rs` | `extract_exif_thumbnails_batch` | 27 | `require_admin` |
| `src-tauri/src/thumbnails.rs` | `generate_full_thumbnails` | 133 | `require_session` |
| `src-tauri/src/thumbnails.rs` | `generate_thumbnail_single` | 200 | `require_session` |
| `src-tauri/src/collections.rs` | `create_collection` | 41 | `require_session` |
| `src-tauri/src/collections.rs` | `rename_collection` | 53 | `require_session` |
| `src-tauri/src/collections.rs` | `delete_collection` | 70 | `require_session` |
| `src-tauri/src/collections.rs` | `add_to_collection` | 83 | `require_session` |
| `src-tauri/src/collections.rs` | `remove_from_collection` | 103 | `require_session` |
| `src-tauri/src/smart_collections.rs` | `create_smart_collection` | 51 | `require_session` |
| `src-tauri/src/smart_collections.rs` | `delete_smart_collection` | 89 | `require_session` |
| `src-tauri/src/drive.rs` | `retry_drive_connection` | 295 | `require_session` |
| `src-tauri/src/drive.rs` | `reveal_drive_in_finder` | 314 | `require_session` |
| `src-tauri/src/opensf_sync.rs` | `sync_image_from_opensf` | 196 | `require_session` |

**Read-only queries that don't mutate** (`get_image`, `query_images`, `get_collections`, `get_audit_log`, etc.) can remain unguarded — they only run after the frontend has rendered the library view, which already requires a session via the `+page.svelte` gate. If you want defence in depth, add `require_session` to those too; cost is one extra mutex acquire per call.

**For `editor::update_image_metadata` specifically** ([editor.rs:59](src-tauri/src/editor.rs)): replace the existing fallback-to-`"local"` with the actual session username. The current code is:

```rust
let actor = current_session(&state)
    .map(|s| s.username)
    .unwrap_or_else(|| "local".to_string());
```

After the session gate, this becomes:

```rust
let session = auth::require_session(&state)?;
let actor = session.username;
```

Same change in `sharing::create_share_link` ([sharing.rs:450-452](src-tauri/src/sharing.rs)).

**Acceptance:**
- `cargo build` clean
- Manual test: log out, call any guarded command from devtools (`__TAURI_INTERNALS__.invoke('reset_catalog')`) — must error with `"Not logged in"`
- Editor account: `invoke('reset_catalog')` must error with `"Admin access required"`

---

### Task 1.3 — Validate UUIDs at the sharing-command boundary

**File:** `src-tauri/src/sharing.rs`

Add after the imports block (around line 18):

```rust
/// Order UUIDs come from the external Laravel API and end up in filesystem
/// paths and S3 keys. Reject anything that isn't a plausible UUID-like
/// identifier so a compromised/malicious API can't induce path traversal.
fn validate_uuid(s: &str) -> Result<(), String> {
    if s.len() < 8 || s.len() > 64
        || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Invalid order identifier".to_string());
    }
    Ok(())
}
```

Call as the first line of `fulfill_order` (after `require_session`) and `fail_order`:

```rust
pub async fn fulfill_order(uuid: String, state: tauri::State<'_, AppState>) -> Result<FulfillResult, String> {
    auth::require_session(&state)?;
    validate_uuid(&uuid)?;
    // ...rest unchanged
}
```

**Acceptance:** unit test in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_uuid_accepts_realistic_shapes() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("ord_abc123").is_ok());
        assert!(validate_uuid("ABCDEF12").is_ok());
    }

    #[test]
    fn validate_uuid_rejects_traversal_and_specials() {
        assert!(validate_uuid("../etc/passwd").is_err());
        assert!(validate_uuid("a/b").is_err());
        assert!(validate_uuid("a b").is_err());
        assert!(validate_uuid("short").is_err());
        assert!(validate_uuid("").is_err());
        assert!(validate_uuid(&"a".repeat(65)).is_err());
    }
}
```

---

### Task 1.4 — Fix CSV formula injection in audit-log export

**File:** `src-tauri/src/editor.rs:285-302`

Replace the existing `csv_escape`:

```rust
/// CSV-escape a single field per RFC 4180 + spreadsheet formula-injection
/// hardening (CWE-1236). If the field starts with one of the formula-trigger
/// chars (=, +, -, @, tab, CR), prepend a single quote so Excel/Numbers/
/// LibreOffice render it as text instead of executing it. Then standard
/// RFC 4180 quoting handles the rest.
fn csv_escape(s: &str) -> String {
    let needs_prefix = s.chars().next().map_or(false, |c| {
        matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r')
    });
    let prefixed = if needs_prefix {
        format!("'{}", s)
    } else {
        s.to_string()
    };
    if prefixed.contains(',') || prefixed.contains('"')
        || prefixed.contains('\n') || prefixed.contains('\r')
    {
        let mut buf = String::with_capacity(prefixed.len() + 2);
        buf.push('"');
        for ch in prefixed.chars() {
            if ch == '"' { buf.push('"'); buf.push('"'); }
            else { buf.push(ch); }
        }
        buf.push('"');
        buf
    } else {
        prefixed
    }
}
```

**Acceptance:** unit test:

```rust
#[cfg(test)]
mod tests {
    use super::csv_escape;

    #[test]
    fn formula_triggers_get_prefixed_quote() {
        assert_eq!(csv_escape("=cmd|'/c calc'!A1"), "\"'=cmd|'/c calc'!A1\"");
        assert_eq!(csv_escape("+1234"), "'+1234");
        assert_eq!(csv_escape("-5"), "'-5");
        assert_eq!(csv_escape("@SUM(A1)"), "'@SUM(A1)");
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("San Francisco"), "San Francisco");
    }

    #[test]
    fn rfc4180_still_works() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("she said \"hi\""), "\"she said \"\"hi\"\"\"");
    }
}
```

---

### Task 1.5 — Stop ExifTool flag-parsing on file path

**File:** `src-tauri/src/editor.rs:393-395`

Before the final `args.push(file_path.clone())` line in `write_metadata_to_file`, insert a `--` arg-terminator:

```rust
// `--` tells exiftool to stop parsing flags. Defends against file paths
// that begin with `-` being misinterpreted as flags.
args.push("--".to_string());
args.push(file_path.clone());
```

**Acceptance:** `cargo build` clean. Manual test (or stubbed test): create a file named `-test.jpg`, ensure `write_metadata_to_file` does not raise a "Unknown option" error from ExifTool.

---

### Task 1.6 — Phase 1 commit

Single commit with all 1.x changes:

```
security: backend auth gates + UUID/CSV/ExifTool input hardening (Phase 1)

- auth.rs: add require_session helper
- gate every state-mutating command on require_session/require_admin
- sharing.rs: validate uuid before using in fs/S3 paths
- editor.rs: CSV-escape formula triggers (CWE-1236)
- editor.rs: -- arg-terminator before file path in exiftool invocation
- tests for csv_escape and validate_uuid

Closes the four most exploitable findings from the 2026-05-03 review:
no-backend-auth, UUID path traversal, CSV formula injection, exiftool
flag-confusion.
```

---

## Phase 2 — Hardening (target: one PR within a week of Phase 1)

These need light schema work + a CSP that won't break the existing UI.

### Task 2.1 — Tauri CSP and asset-protocol scope

**File:** `src-tauri/tauri.conf.json:14-23`

Replace the `security` block with:

```json
"security": {
  "csp": "default-src 'self'; img-src 'self' data: blob: asset: https://asset.localhost; connect-src 'self' ipc: http://ipc.localhost https://*; style-src 'self' 'unsafe-inline'; script-src 'self'; font-src 'self' data:; object-src 'none'; frame-ancestors 'none'",
  "assetProtocol": {
    "enable": true,
    "scope": [
      "$DATA/org.wnp.imagearchive/thumbnails/**",
      "/Volumes/**/*.jpg",
      "/Volumes/**/*.jpeg",
      "/Volumes/**/*.tif",
      "/Volumes/**/*.tiff",
      "/Volumes/**/*.png",
      "/Volumes/**/*.gif",
      "/Volumes/**/*.bmp",
      "/Volumes/**/*.webp",
      "/Volumes/**/*.JPG",
      "/Volumes/**/*.JPEG",
      "/Volumes/**/*.TIF",
      "/Volumes/**/*.TIFF",
      "/Volumes/**/*.PNG",
      "/Volumes/**/*.GIF",
      "/Volumes/**/*.BMP",
      "/Volumes/**/*.WEBP",
      "$HOME/Pictures/**/*.{jpg,jpeg,tif,tiff,png,gif,bmp,webp,JPG,JPEG,TIF,TIFF,PNG,GIF,BMP,WEBP}"
    ]
  }
}
```

**Note on scope:** Tauri 2's glob matcher in some versions does not support brace expansion, hence the explicit per-extension entries for `/Volumes`. Verify against `tauri-utils` glob behavior; consolidate if the version in use supports `{...}`.

**Note on `connect-src https://*`:** the OpenSFHistory and B2 endpoints are admin-configured at runtime. We can't list them statically. The wildcard is a known compromise; the alternative is dynamically rewriting the CSP on settings change, which Tauri 2 supports via `set_csp` but adds complexity. Revisit if the team wants stricter network egress.

**Acceptance:**
- App boots; library view loads thumbnails from the configured archive directory
- Detail view loads full-size images from the archive
- Manual test in devtools: `fetch('file:///etc/passwd')` returns a CSP error
- Manual test: `convertFileSrc('/etc/passwd')` rendered in `<img>` does not load
- Order fulfillment + share dialog still work (S3 + Laravel API egress)

---

### Task 2.2 — Sanitize `catalog_number` at scan time

**File:** `src-tauri/src/scanner.rs`

After the `is_image_file` helper (around line 19), add:

```rust
/// Catalog numbers flow into S3 keys, audit logs, and (eventually) URLs.
/// Restrict to ASCII alphanumeric + `_-.` and reject traversal sequences.
fn sanitize_catalog_number(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.len() > 128 { return None; }
    if trimmed.starts_with('.') || trimmed.contains("..") { return None; }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return None;
    }
    Some(trimmed.to_string())
}
```

In `scan_directory`, replace the `catalog_number` derivation (lines 71-75):

```rust
let stem = entry_path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("");
let catalog_number = match sanitize_catalog_number(stem) {
    Some(c) => c,
    None => {
        eprintln!("scan: skipping {} (invalid catalog number)", entry_path.display());
        continue;
    }
};
```

Note: `total_files` should not be incremented for skipped files. Move the `total_files += 1` line below the catalog-number check.

**Acceptance:** unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::sanitize_catalog_number;

    #[test]
    fn accepts_normal_catalog_numbers() {
        assert_eq!(sanitize_catalog_number("wnp27.4283"), Some("wnp27.4283".into()));
        assert_eq!(sanitize_catalog_number("WNP83-0001"), Some("WNP83-0001".into()));
        assert_eq!(sanitize_catalog_number("img_001"), Some("img_001".into()));
    }

    #[test]
    fn rejects_traversal_and_specials() {
        assert_eq!(sanitize_catalog_number("../etc/passwd"), None);
        assert_eq!(sanitize_catalog_number(".hidden"), None);
        assert_eq!(sanitize_catalog_number("a/b"), None);
        assert_eq!(sanitize_catalog_number("a b"), None);
        assert_eq!(sanitize_catalog_number(""), None);
        assert_eq!(sanitize_catalog_number(&"a".repeat(129)), None);
    }
}
```

---

### Task 2.3 — Validate `reveal_drive_in_finder` target

**File:** `src-tauri/src/drive.rs:314-330`

Replace the body of `reveal_drive_in_finder`:

```rust
#[tauri::command]
pub fn reveal_drive_in_finder(state: State<AppState>) -> Result<(), String> {
    crate::auth::require_session(&state)?;

    let status = state
        .drive_state
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let target = status
        .mount_point
        .clone()
        .or(status.source_directory.clone())
        .ok_or_else(|| "No drive path configured".to_string())?;

    let path = std::path::PathBuf::from(&target);
    if !path.is_dir() {
        return Err(format!("Drive path is not a directory: {}", target));
    }

    // `--` halts flag parsing in /usr/bin/open so paths beginning with `-`
    // can't be misinterpreted as options.
    std::process::Command::new("open")
        .arg("--")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}
```

**Acceptance:** manual test — `reveal_drive_in_finder` with a valid mount opens Finder; with `source_directory` set to `https://evil.com` (via `set_setting`), errors out with "not a directory".

---

### Task 2.4 — Transactional admin-count check in user_management

**File:** `src-tauri/src/user_management.rs`

Rewrite `update_user_role` (lines 117-151) and `delete_user` (lines 174-192) so the count + mutation are inside a single transaction. Lock the `db` guard mutably:

```rust
#[tauri::command]
pub fn update_user_role(
    user_id: i64,
    role: UserRole,
    state: State<AppState>,
) -> Result<(), String> {
    let _admin_session = require_admin(&state)?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;

    let (_target_username, current_role) = {
        let row = tx.query_row(
            "SELECT username, role FROM users WHERE id = ?1",
            params![user_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        ).map_err(|_| "User not found".to_string())?;
        let role = UserRole::from_db_str(&row.1).ok_or_else(|| "Unknown role".to_string())?;
        (row.0, role)
    };

    if current_role == UserRole::Admin && role != UserRole::Admin {
        let admins: i64 = tx.query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'admin'",
            [],
            |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        if admins <= 1 {
            return Err("Can't demote the last remaining admin. Promote another user first.".to_string());
        }
    }

    tx.execute(
        "UPDATE users SET role = ?1 WHERE id = ?2",
        params![role.as_str(), user_id],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
```

Apply the same pattern to `delete_user`. **Note** the change from `let db = ...` to `let mut db = ...` — `Connection::transaction()` requires `&mut self`.

**Acceptance:** existing manual flow (promote/demote, delete user) still works. No new tests required for this task; SQLite's serialised-transaction semantics give us the invariant.

---

### Task 2.5 — Whitelisted `get_setting` and split secret keys

**Files:** `src-tauri/src/settings.rs`, `src/lib/commands/settings.ts`

In `settings.rs`, define the secret-key set and add a public-only command:

```rust
/// Settings that contain credentials. `get_setting`/`set_setting` for these
/// keys requires admin role; `get_public_setting` rejects them entirely.
const SECRET_KEYS: &[&str] = &[
    "s3_secret_key",
    "s3_access_key",
    "laravel_api_token",
];

fn is_secret(key: &str) -> bool {
    SECRET_KEYS.contains(&key)
}

#[tauri::command]
pub fn get_public_setting(key: String, state: tauri::State<AppState>) -> Result<Option<String>, String> {
    if is_secret(&key) {
        return Err(format!("Setting '{}' is not public", key));
    }
    inner_get_setting(&key, &state)
}

#[tauri::command]
pub fn get_setting(key: String, state: tauri::State<AppState>) -> Result<Option<String>, String> {
    if is_secret(&key) {
        crate::auth::require_admin(&state)?;
    }
    inner_get_setting(&key, &state)
}

fn inner_get_setting(key: &str, state: &tauri::State<AppState>) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
```

`set_setting` already gets `require_admin` from Task 1.2.

Register `get_public_setting` in `lib.rs:39` alongside `get_setting`.

In `src/lib/commands/settings.ts`:

```ts
export async function getPublicSetting(key: string): Promise<string | null> {
  return invoke('get_public_setting', { key });
}
```

Audit the frontend for callers of `getSetting` that don't need admin role. Move them to `getPublicSetting`:

| File | Setting | New call |
| --- | --- | --- |
| `src/routes/+page.svelte` | `source_directory` | `getPublicSetting` |
| `src/lib/components/settings/pages/GeneralPage.svelte` | `source_directory` | `getPublicSetting` |
| `src/lib/components/settings/pages/SharingPage.svelte` | `resolution_*_px` | `getPublicSetting` |
| `src/lib/stores/inactivityTimeout.ts` | `inactivity_timeout_minutes` | `getPublicSetting` |
| `src/lib/components/settings/pages/ApiPage.svelte` | all of them | keep `getSetting` (admin-only page) |

**Acceptance:**
- App boots normally for both admin and editor roles
- Editor role: opening `ApiPage.svelte` errors out (it shouldn't be reachable for editors anyway via the existing route guard, but defence in depth)
- Devtools test: `invoke('get_public_setting', { key: 's3_secret_key' })` errors with `"Setting 's3_secret_key' is not public"`

---

### Task 2.6 — Phase 2 commit

```
security: CSP, scoped asset protocol, input sanitisation (Phase 2)

- tauri.conf.json: strict CSP + asset scope limited to thumbnails + image dirs
- scanner.rs: sanitize_catalog_number; reject traversal at ingest
- drive.rs: validate target before /usr/bin/open + arg-terminator
- user_management.rs: transactional admin-count check
- settings.rs: split get_setting from get_public_setting; secret-key allowlist
- frontend: route non-secret reads through getPublicSetting
```

---

## Phase 3 — Credential migration + auth maturation (target: next sprint)

> **Tasks 3.1–3.3 reverted 2026-05-04.** See "Keychain reversal" in the resolved-decisions section. Tasks 3.4 (password policy), 3.5 (rate limiting), and 3.6 (case-insensitive usernames) shipped as planned.

### Task 3.1 — ~~Move S3 + API credentials to macOS Keychain~~ (REVERTED)

**Cargo:** add `keyring = "3"` to `src-tauri/Cargo.toml` dependencies.

**New file:** `src-tauri/src/secrets.rs`

```rust
//! Encrypted credential storage backed by the OS keychain.
//!
//! On macOS this is the user's login keychain. The service name is
//! "org.wnp.imagearchive" and the account name is the setting key
//! ("s3_secret_key", etc.). Falls back to a clear error rather than
//! plaintext storage if the keychain is unavailable.

use keyring::Entry;

const SERVICE: &str = "org.wnp.imagearchive";

const SECRET_KEYS: &[&str] = &[
    "s3_secret_key",
    "s3_access_key",
    "laravel_api_token",
];

pub fn is_secret(key: &str) -> bool {
    SECRET_KEYS.contains(&key)
}

pub fn get(key: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set(key: &str, value: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
    if value.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        entry.set_password(value).map_err(|e| e.to_string())
    }
}

pub fn delete(key: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
```

Register in `lib.rs:11`: `pub mod secrets;`

---

### Task 3.2 — Migration: move existing plaintext secrets into Keychain

**File:** `src-tauri/src/db.rs`

Add a new function `migrate_secrets_to_keychain` and call it after `run_migrations`:

```rust
/// Migration 004 (Plan 11): move credential settings from `app_settings`
/// into the macOS keychain. Idempotent; a rerun is a no-op once the rows
/// are gone.
pub fn migrate_secrets_to_keychain(conn: &Connection) -> Result<()> {
    let secret_keys = ["s3_secret_key", "s3_access_key", "laravel_api_token"];
    for key in secret_keys {
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                [key],
                |r| r.get(0),
            )
            .ok();
        if let Some(v) = value {
            if !v.is_empty() {
                if let Err(e) = crate::secrets::set(key, &v) {
                    eprintln!("keychain migration: failed to store {}: {}", key, e);
                    continue;
                }
            }
            // Remove the plaintext row whether we stored it or it was empty.
            let _ = conn.execute(
                "DELETE FROM app_settings WHERE key = ?1",
                [key],
            );
        }
    }
    Ok(())
}
```

Wire into `init_db`:

```rust
pub fn init_db() -> Result<Connection> {
    let db_path = get_db_path();
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    run_migrations(&conn)?;
    migrate_secrets_to_keychain(&conn)?;
    Ok(conn)
}
```

---

### Task 3.3 — Route secret reads/writes through `secrets::*`

**File:** `src-tauri/src/settings.rs`

Update `inner_get_setting` (or replace `get_setting`) and `set_setting`:

```rust
fn inner_get_setting(key: &str, state: &tauri::State<AppState>) -> Result<Option<String>, String> {
    if crate::secrets::is_secret(key) {
        return crate::secrets::get(key);
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // ...existing app_settings query
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: tauri::State<AppState>) -> Result<(), String> {
    crate::auth::require_admin(&state)?;
    if crate::secrets::is_secret(&key) {
        return crate::secrets::set(&key, &value);
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
```

The `read_setting`/`read_setting_opt` helpers in `sharing.rs` and `opensf_sync.rs` (M1) need the same dispatch. **This is a good moment to do M1** — extract a single helper into `db.rs` that handles both regular settings and keychain secrets.

**File:** `src-tauri/src/db.rs` add:

```rust
pub fn read_setting(conn: &Connection, key: &str) -> Result<String, String> {
    if crate::secrets::is_secret(key) {
        return crate::secrets::get(key)
            .map_err(|e| format!("Failed to read secret '{}': {}", key, e))?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("Secret '{}' is not configured", key));
    }
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .map_err(|_| format!("Setting '{}' is not configured", key))
    .and_then(|v| if v.is_empty() {
        Err(format!("Setting '{}' is empty", key))
    } else {
        Ok(v)
    })
}

pub fn read_setting_opt(conn: &Connection, key: &str) -> Option<String> {
    if crate::secrets::is_secret(key) {
        return crate::secrets::get(key).ok().flatten().filter(|s| !s.is_empty());
    }
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|s| !s.is_empty())
}
```

Delete the duplicated copies in `sharing.rs:35-59` and `opensf_sync.rs:87-111`. Update imports.

**Acceptance:**
- Existing install: app boots, settings page shows the previously stored S3 secret in the password field, fulfill-order still works
- Fresh install: setting credentials writes to keychain, app_settings table remains empty for those keys
- Manual check via `security find-generic-password -s org.wnp.imagearchive -a s3_secret_key -w`

---

### Task 3.4 — Stricter password policy

**File:** `src-tauri/src/auth.rs:24, 75-88`

```rust
const MIN_PASSWORD_LEN: usize = 12;

pub fn hash_password(password: &str) -> Result<String, String> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "Password must be at least {} characters",
            MIN_PASSWORD_LEN
        ));
    }
    // Reject common-password patterns. Cheap heuristic — full HIBP-list
    // integration is a future improvement.
    let lower = password.to_lowercase();
    if lower == password && password.chars().all(|c| c.is_ascii_alphanumeric()) {
        // All lowercase alphanumeric — too predictable. Demand at least one
        // uppercase letter, digit-mix, or symbol.
        if !password.chars().any(|c| c.is_ascii_digit())
            || !password.chars().any(|c| c.is_ascii_alphabetic())
        {
            return Err("Password must include both letters and digits".to_string());
        }
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {}", e))?
        .to_string())
}
```

Update the LoginScreen / UsersPage helper text to mention the new minimum.

**Acceptance:** unit test:

```rust
#[test]
fn rejects_short_passwords() {
    assert!(hash_password("short").is_err());
    assert!(hash_password("eleven_chars").is_err()); // 12 below
}

#[test]
fn accepts_strong_passwords() {
    assert!(hash_password("correcthorsebattery9").is_ok());
    assert!(hash_password("PassPhrase2026!").is_ok());
}
```

---

### Task 3.5 — Login rate limiting

**File:** `src-tauri/src/db.rs`, `src-tauri/src/auth.rs`

In `db.rs`, extend `AppState`:

```rust
use std::collections::HashMap;
use std::time::Instant;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub drive_state: Mutex<DriveStatus>,
    pub current_user: Mutex<Option<UserSession>>,
    /// Failed-login tracker keyed by lowercased username.
    /// Tuple is (failed_count, first_failure_at). Reset on success or after
    /// the lockout window.
    pub login_attempts: Mutex<HashMap<String, (u32, Instant)>>,
}
```

Initialize in `lib.rs:29-33`:

```rust
.manage(AppState {
    db: Mutex::new(db),
    drive_state: Mutex::new(DriveStatus::default()),
    current_user: Mutex::new(None),
    login_attempts: Mutex::new(HashMap::new()),
})
```

In `auth.rs:174` rewrite the start of `login`:

```rust
const MAX_FAILURES: u32 = 5;
const LOCKOUT_WINDOW: Duration = Duration::from_secs(300); // 5 min
const LOCKOUT_DURATION: Duration = Duration::from_secs(60);

#[tauri::command]
pub fn login(...) -> Result<UserSession, String> {
    let username_key = username.trim().to_lowercase();

    // Lockout check
    {
        let mut attempts = state.login_attempts.lock().map_err(|e| e.to_string())?;
        if let Some((count, first_at)) = attempts.get(&username_key).copied() {
            let elapsed = first_at.elapsed();
            if count >= MAX_FAILURES && elapsed < LOCKOUT_DURATION {
                return Err("Too many failed attempts. Try again in a minute.".to_string());
            }
            if elapsed > LOCKOUT_WINDOW {
                attempts.remove(&username_key);
            }
        }
    }

    // ...existing DB lookup + verify_password logic...

    // On failure (replace existing return-Err):
    if !verify_password(&password, &password_hash) {
        let mut attempts = state.login_attempts.lock().map_err(|e| e.to_string())?;
        let entry = attempts.entry(username_key.clone()).or_insert((0, Instant::now()));
        if entry.1.elapsed() > LOCKOUT_WINDOW {
            *entry = (0, Instant::now());
        }
        entry.0 += 1;
        return Err("Invalid username or password".to_string());
    }

    // On success: clear the counter
    {
        let mut attempts = state.login_attempts.lock().map_err(|e| e.to_string())?;
        attempts.remove(&username_key);
    }

    // ...existing session creation...
}
```

**Acceptance:** manual test — 5 wrong passwords on the same username produce a lockout error on the 6th attempt; correct password 60s later succeeds.

---

### Task 3.6 — Username case-insensitivity

**File:** `src-tauri/sql/schema.sql`, new migration file `src-tauri/sql/migration_004_username_nocase.sql`

```sql
-- Migration 004: case-insensitive username lookups.
-- The original schema declared UNIQUE without COLLATE NOCASE, allowing
-- "Alice" and "alice" to coexist. Add a unique index on LOWER(username)
-- to enforce case-insensitivity going forward without rewriting the table.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username_nocase
  ON users(LOWER(username));
```

In `db.rs::run_migrations`, after the existing migrations:

```rust
// Migration 004 (Plan 11): username case-insensitivity.
let has_index: bool = conn
    .query_row(
        "SELECT COUNT(*) FROM sqlite_schema WHERE type='index' AND name='idx_users_username_nocase'",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false);
if !has_index {
    conn.execute_batch(include_str!("../sql/migration_004_username_nocase.sql"))?;
}
```

In `auth.rs:185, 209`, change the WHERE clauses:

```sql
WHERE LOWER(username) = LOWER(?1)
```

In `user_management.rs::create_user` ([line 90](src-tauri/src/user_management.rs)) the existing UNIQUE catch will still trigger because the new index covers the case-insensitive collision. Update the error string to reflect that:

```rust
.map_err(|e| {
    if e.to_string().contains("UNIQUE") || e.to_string().contains("idx_users_username_nocase") {
        "Username already taken (matches existing user, ignoring case)".to_string()
    } else {
        e.to_string()
    }
})
```

**Acceptance:** trying to create `Alice` when `alice` exists errors out with the new message. Logging in as either case authenticates the same user.

---

### Task 3.7 — Phase 3 commit set

Three commits, in order:

1. `security: keychain-backed credential storage with auto-migration`
2. `security: stricter password policy + login rate limiting`
3. `security: case-insensitive usernames`

---

## Phase 4 — Tech debt + reliability (backlog)

These are not security-blocking but reduce future regression risk.

### Task 4.1 — Replace `unchecked_transaction` with `transaction`

**Files:** [editor.rs:64](src-tauri/src/editor.rs), [sharing.rs:282,557](src-tauri/src/sharing.rs), [scanner.rs:95,176](src-tauri/src/scanner.rs), [collections.rs:89,109](src-tauri/src/collections.rs), [metadata.rs:151](src-tauri/src/metadata.rs)

Each call site:
1. Change `let db = state.db.lock()...` to `let mut db = state.db.lock()...`
2. Change `db.unchecked_transaction()` to `db.transaction()`

Some sites pass the connection by `&` reference into helper fns (`scanner.rs::create_archive_collections` takes `&rusqlite::Connection`). In those cases, change the helper to take `&mut rusqlite::Connection` or restructure so the transaction lives entirely within the calling function.

**Acceptance:** `cargo build` clean. No behavioural change expected.

---

### Task 4.2 — Extract HTTP-client builder + setting helpers

**Already completed in Task 3.3** for `read_setting`/`read_setting_opt`. The `build_authed_client` duplicate ([sharing.rs:68](src-tauri/src/sharing.rs) = [opensf_sync.rs:73](src-tauri/src/opensf_sync.rs)) deserves the same treatment.

**New file:** `src-tauri/src/http.rs`

```rust
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};

pub fn build_authed_client(token: Option<&str>) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    if let Some(t) = token {
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", t)) {
            headers.insert(AUTHORIZATION, val);
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
```

Register in `lib.rs:11`. Remove the duplicates from `sharing.rs` and `opensf_sync.rs`; update imports.

---

### Task 4.3 — URL composition with proper encoding

**Files:** `src-tauri/src/sharing.rs`, `src-tauri/src/opensf_sync.rs`

Replace ad-hoc `format!("{}/segment/{}", base, value)` with `reqwest::Url::parse(&base)?.join("segment/")?.join(&value)?` or `Url::path_segments_mut()`. This handles trailing-slash normalisation and percent-encoding correctly.

Example for `sharing.rs:155`:

```rust
let mut url = reqwest::Url::parse(&api_url)
    .map_err(|e| format!("Invalid API URL: {}", e))?;
{
    let mut segs = url.path_segments_mut().map_err(|_| "Invalid API URL")?;
    segs.pop_if_empty().push("image-requests").push(&uuid);
}
```

---

### Task 4.4 — Enforce HTTPS on Laravel API URL

**File:** `src-tauri/src/settings.rs`

In `set_setting`, when key is `laravel_api_url`:

```rust
if key == "laravel_api_url" && !value.is_empty() {
    if !value.starts_with("https://") && !value.starts_with("http://localhost") {
        return Err("API URL must use HTTPS".to_string());
    }
}
```

Same check at the top of every command in `sharing.rs` / `opensf_sync.rs` that reads `laravel_api_url`, as a defence-in-depth.

---

### Task 4.5 — Replace `eprintln!` with a proper logger

**Cargo:** add `log = "0.4"` and `env_logger = "0.11"` (or `tracing` + `tracing-subscriber`).

In `lib.rs::run`, before `tauri::Builder`:

```rust
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .init();
```

Replace every `eprintln!` in `opensf_sync.rs`, `metadata.rs`, `thumbnails.rs` with `log::warn!` or `log::debug!` as appropriate. URLs and bodies move to `log::debug!` so they only appear in dev builds.

---

### Task 4.6 — `walkdir` error surfacing

**File:** `src-tauri/src/scanner.rs`

Add a `walk_errors` field to `ScanResult` (and `models.rs::ScanResult`). Count `entry.is_err()` cases instead of silently filtering them.

---

### Task 4.7 — Migration runner with version table

**File:** new `src-tauri/sql/schema.sql` addition + `db.rs`

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Refactor `db.rs::run_migrations` to:
1. Read applied versions
2. For each migration file in `sql/migrations/NNN_*.sql` (sorted), if version not applied: execute + insert version
3. Drop the existing ad-hoc `add_column_if_missing` and trigram-detection paths

Migrate the existing implicit migrations (`migration_001`, `migration_003`, `migration_004` from this plan) into numbered files under `sql/migrations/`.

---

### Task 4.8 — Drive poller shutdown

**File:** `src-tauri/src/drive.rs`

Store a `JoinHandle` and an `Arc<AtomicBool>` shutdown flag in `AppState`. Wire to `app.on_window_event(WindowEvent::Destroyed, ...)` in `lib.rs::run`. Low priority — only matters if/when the app supports restart-in-place.

---

### Task 4.9 — Test scaffolding

Add `cargo test` invocation to CI (or to `bun run check`). At minimum cover:
- `csv_escape` (Task 1.4)
- `validate_uuid` (Task 1.3)
- `sanitize_catalog_number` (Task 2.2)
- `hash_password` rejecting weak inputs (Task 3.4)
- `auth::require_admin`/`require_session` returning Err when no session

---

## Verification checklist (run after each phase)

- `cargo build --release` clean
- `bun run check` clean
- App boots to login screen
- Login as admin → library view loads → thumbnails render
- Login as editor → cannot reach Settings → cannot call admin commands from devtools
- Fulfill an order end-to-end (requires a configured B2 + Laravel test instance)
- Create an ad-hoc share end-to-end
- Reset catalog from Settings; re-import
- After Phase 3: confirm `app_settings` table contains no `s3_*` or `laravel_api_token` rows; confirm `security find-generic-password -s org.wnp.imagearchive -a s3_secret_key` returns the value

## Out of scope for this plan

- HIBP-list integration for password strength (heuristic check is good enough for now)
- Encrypted SQLite (SQLCipher) — single-user macOS, Keychain covers the credential class
- 2FA / hardware keys — overkill for this app's threat model
- Audit-log integrity protection (HMAC chains) — interesting but separate
- Network egress allowlist via dynamic CSP rewrites
- Penetration test against a deployed build

These are listed in the review document but deliberately deferred. Revisit if the threat model changes (e.g. multi-user deployment, public exposure, regulatory requirement).
