# Plan 10 — Local user management

> **Status:** Active. Decisions locked 2026-05-01. Not in the original 9-plan roadmap; surfaced after Plan 6 shipped as a workflow need: app sits on a shared work computer; we want per-user attribution on edits without external auth infrastructure.
>
> **Depends on:** Plan 6 (uses the same full-screen overlay pattern as DriveDisconnectedScreen).

## Goal

Local username/password auth that gates app access and attributes audit-log entries to the active user. Two roles: **admin** (full access incl. Settings) and **editor** (no Settings). Auto-logout after 15 min of inactivity (configurable). Admin manages all users.

## Resolved decisions

| Question | Decision |
| --- | --- |
| Bootstrap | **Empty `users` table → "Create admin" screen** on first launch. Same path covers fresh installs and the existing populated production DB. After admin creation, app auto-logs-in the new admin and proceeds normally. |
| Editor restrictions | **Hide Settings only.** Editors retain full edit/share/fulfill capability — every action attributed to them in the audit log. No write-blocking for editors. |
| User indicator + logout location | **Sidebar footer + `⌘⇧L` shortcut.** Username · role chip at the very bottom of the sidebar (below `ActivityCard`); click opens a small popover with "Change password" + "Log out". `⌘⇧L` works globally. |
| Inactivity behavior | **Hard logout, 15 min default.** No lock-screen middle state. After timeout the app shows the login form same as a manual logout. Threshold lives in `app_settings` key `inactivity_timeout_minutes`. |

## Scope

**Backend:**
- New `users` table (id, username unique, password_hash, role, created_at, last_login_at).
- New `auth.rs` module: argon2id password hashing, `UserSession` struct, current-user state in `AppState`.
- New commands: `is_setup_required`, `create_first_admin`, `login`, `logout`, `get_current_user`, plus admin-only CRUD: `list_users`, `create_user`, `update_user_role`, `update_user_password`, `delete_user`.
- `editor::update_image_metadata` reads the current session and writes `audit_log.changed_by = username` (falls back to `'local'` if no session — defensive).

**Frontend:**
- `currentUser` writable store + `currentUserRole` derived.
- `LoginScreen.svelte` — single component handling both the "create first admin" bootstrap and normal login (decided by an `is_setup_required` check on mount).
- `UserMenu.svelte` — sidebar footer chip + popover (Change password modal + Log out).
- `UsersPage.svelte` — Settings → Users page: list with role / last-login / last-activity, "Add user" modal, per-row actions (change password, change role, delete with safety: can't delete yourself or last admin). Inactivity timeout setting at top.
- Inactivity tracker — global mousemove/keydown/click listener resets a 15-min timer; on expiry, calls `logout`.
- `⌘⇧L` global shortcut → `logout`.
- `+page.svelte` — when no current user (and `is_setup_required` resolved), render `LoginScreen` as a full-area overlay above sidebar (i.e., entire window is the login form during this state, since they shouldn't be navigating yet).
- Editor role: `Sidebar.svelte` hides the Settings item; `+page.svelte` redirects `currentView === 'settings'` → `'library'` if the user is an editor; ⌘; settings shortcut becomes a no-op.

## Out of scope

- Password complexity rules (let admin pick whatever; clear minimum 6 chars but no patterns enforced)
- Password recovery / reset emails — admin resets passwords directly
- Self-signup
- Multi-factor auth
- Session sharing across devices (this is a single-machine app)
- Migrating historical `audit_log.changed_by = 'local'` entries — they stay as-is; new entries get attributed
- "Switch user" without logout — must log out, then back in

## Architecture

```
                ┌────────────────────────────────┐
                │  Tauri commands (auth.rs)      │
                │  ─ argon2id verify             │
                │  ─ writes UserSession into     │
                │    AppState.current_user       │
                │  ─ emits "auth:changed" event  │
                └─────────┬──────────────────────┘
                          │ Tauri event
                          ▼
             ┌─────────────────────────────┐
             │  currentUser store          │
             │  + currentUserRole derived  │
             └─────────┬───────────────────┘
                       │
        ┌──────────────┼─────────────────────────┐
        ▼              ▼                         ▼
  Sidebar           +page.svelte             Inactivity
  (UserMenu       (LoginScreen overlay        tracker
   in footer;      when !currentUser;       (in +page;
   hides           Settings hidden          15min idle
   Settings        for editors)             → logout)
   for editors)
```

## Tasks

Each phase has a verification gate. Run `cargo check` after Rust phases and `bun run check` after Svelte phases.

### Phase A — Schema + dependencies

**A1.** `src-tauri/sql/schema.sql`: add a `users` table near the bottom:
```sql
CREATE TABLE IF NOT EXISTS users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    username        TEXT    NOT NULL UNIQUE,
    password_hash   TEXT    NOT NULL,
    role            TEXT    NOT NULL CHECK (role IN ('admin','editor')),
    created_at      TEXT    DEFAULT (datetime('now')),
    last_login_at   TEXT
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
```

**A2.** `src-tauri/Cargo.toml`: `argon2 = "0.5"` (this includes `password_hash` facade and re-exports `OsRng` via `rand_core`).

**A3.** Verify `cargo check` clean.

### Phase B — Auth module

**B1.** Create `src-tauri/src/auth.rs`:
- `UserRole` enum (Admin | Editor) with serde `rename_all = "lowercase"`.
- `UserSession { user_id, username, role, login_at_ms, last_activity_at_ms }` (camelCase serde).
- `User { id, username, role, created_at, last_login_at }` (no password hash exposed).
- `hash_password`, `verify_password` helpers using argon2id.
- Commands: `is_setup_required`, `create_first_admin(username, password)`, `login(username, password)`, `logout()`, `get_current_user()`.
- All session-mutating commands emit `auth:changed` Tauri event with the new state.

**B2.** Add `pub current_user: Mutex<Option<UserSession>>` to `AppState` in `db.rs` (default `None`).

**B3.** Create `src-tauri/src/user_management.rs` with admin-gated commands:
- `list_users() -> Vec<User>`
- `create_user(username, password, role)`
- `update_user_role(user_id, role)`
- `update_user_password(user_id, new_password)`  (admin can change any; non-admin can only change their own — gate on session)
- `delete_user(user_id)`  (deny if it's the current session OR last admin)

Each admin command checks `state.current_user.lock()` for `role == Admin`; returns `Err("Admin access required")` otherwise.

**B4.** Wire into `lib.rs`: `pub mod auth; pub mod user_management;` + register all new commands + initialize `current_user: Mutex::new(None)` in `AppState`.

**B5.** Update `editor::update_image_metadata`: read `state.current_user`, write `changed_by = session.username` (or `'local'` if no session — defensive for the rare case it gets called pre-auth).

### Phase C — Frontend foundations

**C1.** `src/lib/commands/auth.ts` and `src/lib/commands/users.ts`: thin wrappers + types.

**C2.** `src/lib/stores/currentUser.ts`:
- `currentUser` writable<UserSession | null>
- `currentUserRole` derived (admin/editor/null)
- `isAdmin` derived (boolean)
- `setupRequired` writable<boolean | null>  (null = unknown, true/false after probe)
- `initAuthListener()` — on mount: call `is_setup_required` + `get_current_user`, subscribe to `auth:changed` events.

**C3.** Inactivity tracker (`src/lib/utils/inactivityTimer.ts`):
- `installInactivityTimer({ getTimeoutMs, onExpired }) => uninstall`
- Listens to mousemove / keydown / click / scroll on document
- Throttled reset (no more than once per 5s)
- Standard pattern: setTimeout, clearTimeout, on event reset

### Phase D — UI components

**D1.** `src/lib/components/auth/LoginScreen.svelte`:
- Single component, two modes: "create-first-admin" vs "login"
- Mode chosen by `setupRequired` value
- Bootstrap mode: shows "Create your first admin" heading, username + password + confirm password inputs, "Create admin" button
- Login mode: shows "Sign in" heading, username + password, "Sign in" button
- Renders inside an `absolute inset-0 z-50` overlay over the entire window (above WindowChrome? no — chrome stays for drag region; overlay covers everything else)
- After successful login or admin creation, store updates and overlay disappears

**D2.** `src/lib/components/auth/UserMenu.svelte`:
- Sidebar footer chip: avatar (initials) + username + role chip
- Click → bits-ui Popover with: "Change password" (modal) + "Log out"
- Change password modal: current password + new + confirm, validates locally then calls `update_user_password`

**D3.** `src/lib/components/settings/pages/UsersPage.svelte`:
- Top: inactivity timeout setting (number input, minutes, default 15) — saves on blur to `inactivity_timeout_minutes` setting
- Section: "Users" with table-like list: username, role (dropdown — admin only can change), last login, actions (change password, delete)
- "Add user" button → modal (username, password, role)
- Self-row: deletion + role-change disabled with tooltip
- Last-admin protection: if admin would be deleted/downgraded leaving zero admins, block + show error

### Phase E — Wire-up

**E1.** `src/routes/+page.svelte`:
- Init `initAuthListener()` on mount
- New top-level conditional: if `setupRequired === null` → loading spinner; if `currentUser === null` → render `<LoginScreen />` covering everything except `WindowChrome`; else render normal app shell
- Install `installInactivityTimer({ getTimeoutMs: () => $inactivityTimeoutMs, onExpired: logout })` on mount; uninstall on destroy
- Add `⌘⇧L` to existing `installShortcuts` cmdKey block: maps to `logout()`
- Editor guard: if `$currentUserRole === 'editor'` and `$currentView === 'settings'`, force back to `'library'`

**E2.** `src/lib/components/layout/Sidebar.svelte`:
- Conditional render: hide Settings `<SideItem>` if editor
- Mount `<UserMenu />` at the bottom (below ActivityCard)

**E3.** `src/lib/components/settings/SettingsNav.svelte`: ensure `users` page is wired correctly (page exists now).

**E4.** Inactivity timeout: read setting on app boot into a writable store. Refresh whenever the setting changes (via UsersPage save).

### Phase F — Verification

- `cargo check` clean
- `bun run check` clean
- Manual smoke:
  1. Wipe `users` table (or fresh install) → app shows "Create your first admin" → create → auto-logged in as admin
  2. Log out → see login form → log in same admin → back to library
  3. Settings → Users → add an editor user → log out → log in as editor → Settings sidebar item not visible, `⌘;` does nothing, navigating to settings programmatically lands on library
  4. As editor, edit a metadata field → audit log entry shows editor's username
  5. Sit idle 15 min → app logs out automatically
  6. Change inactivity setting to 1 min → logs out faster (don't actually wait, just verify it reads the value)
  7. `⌘⇧L` from any view → immediate logout
  8. Try to delete last admin via UsersPage → error displayed
  9. Try to change own role via UsersPage → button disabled
- Commit message: `Plan 10: Local user management (login + roles + auto-logout)`

## Risks + open considerations

1. **Argon2 verification cost.** Argon2id at default params is intentionally slow (~100ms). A user typing "wrong password" 5 times in a row burns half a second. Acceptable for this scale; if it becomes a UX issue, we can lower memory/iterations, but the security posture matches good practice.

2. **No rate limiting.** This is local; if someone has physical access to the machine they can brute-force forever. Mitigated by argon2 cost factor. We're trusting the work-computer threat model.

3. **Session in RAM only.** Closing the app = logged out. Re-launch requires sign-in. This is intended (matches "step away → app logs out"). If a user complains about losing state after restart we could persist a refresh token, but that's another can of worms.

4. **`current_user` race during a long-running command.** A command that holds the DB lock for a long time (bulk import) could miss a concurrent logout. Acceptable: long commands complete and attribute to whoever started them. The next command after logout sees no session.

5. **Editor vs admin — what about `reset_catalog`?** Currently exposed in Settings. Since editors don't see Settings, they can't trigger it. No backend gate needed — admin-only by virtue of UI access.

6. **Audit log historical entries.** `changed_by = 'local'` from before this plan shipped stays. Filter UI in audit log already shows this; it's accurate ("done before user tracking existed").

7. **Inactivity tracker false-fires.** If app is unfocused for 15min while user is just thinking, they'll be logged out. We could pause the timer when the window loses focus (via Tauri events) but that defeats the point. Default behavior matches the request.

8. **First-admin race.** If two windows opened simultaneously hit `is_setup_required` they could both try to create. Tauri runs single-window, so N/A.

## Estimated size

**M.** Roughly 6–8 hours focused. Backend (~2–3h: schema, auth, user CRUD, editor wiring), Frontend (~3–4h: store, login screen, user menu, users page, inactivity tracker, role gating), wire-up + smoke (~1–2h).
