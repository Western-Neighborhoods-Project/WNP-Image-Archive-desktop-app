// Local user authentication (Plan 10).
//
// Argon2id password hashing, in-memory session in AppState. Two roles:
// - admin: full access including Settings page
// - editor: blocked from Settings (UI-only gate; backend doesn't restrict
//   their actual operations — they can edit metadata, fulfill orders, etc.,
//   and every action is attributed to them in the audit log).
//
// Bootstrap: when the users table is empty, the only allowed auth call is
// `create_first_admin`. After that, normal `login` / `logout` / `get_current_user`
// take over.
//
// Session lives in AppState.current_user (Mutex<Option<UserSession>>). It's
// pure RAM — closing the app drops it. Inactivity timeout is enforced
// frontend-side; this module just stores who's logged in.

use crate::db::AppState;
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

const MIN_PASSWORD_LEN: usize = 12;
/// Minimum number of distinct characters. Rejects degenerate low-entropy
/// strings like "Aaaaaaaaaaaa" that slip past the letter/digit-mix rule.
const MIN_DISTINCT_CHARS: usize = 5;
const MAX_LOGIN_FAILURES: u32 = 5;
/// Window in which failed-login counts accumulate. Past this point, the
/// counter resets even without a successful login.
const LOCKOUT_WINDOW: Duration = Duration::from_secs(300);
/// How long the user is locked out after `MAX_LOGIN_FAILURES`.
const LOCKOUT_DURATION: Duration = Duration::from_secs(60);

/// Outcome of checking the failed-login counter before an attempt.
#[derive(Debug, PartialEq, Eq)]
enum LockoutCheck {
    /// Still inside the lockout window — reject the attempt.
    Locked,
    /// Counter is stale (lockout served, or the window aged out) — clear it and
    /// let this attempt start fresh.
    Reset,
    /// No relevant history — proceed.
    Proceed,
}

/// Decide what to do with an existing failed-login counter.
///
/// The previous inline logic left a hole: the lockout only fired while
/// `elapsed < LOCKOUT_DURATION` (60s), but the counter wasn't cleared until
/// `elapsed > LOCKOUT_WINDOW` (300s) and `first_at` never advanced — so from
/// 60s to 300s the counter stayed `>= MAX` yet was ignored, allowing unlimited
/// guesses. Here, once the lockout has been served we RESET (giving a fresh
/// budget that will re-lock after another `MAX` failures), bounding the rate to
/// `MAX_LOGIN_FAILURES` attempts per `LOCKOUT_DURATION` instead.
fn evaluate_lockout(count: u32, elapsed: Duration) -> LockoutCheck {
    if count >= MAX_LOGIN_FAILURES {
        if elapsed < LOCKOUT_DURATION {
            LockoutCheck::Locked
        } else {
            LockoutCheck::Reset
        }
    } else if elapsed > LOCKOUT_WINDOW {
        LockoutCheck::Reset
    } else {
        LockoutCheck::Proceed
    }
}

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Editor,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Editor => "editor",
        }
    }
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(UserRole::Admin),
            "editor" => Some(UserRole::Editor),
            _ => None,
        }
    }
}

/// Active session. Lives in AppState. Frontend-visible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSession {
    pub user_id: i64,
    pub username: String,
    pub role: UserRole,
    pub login_at_ms: i64,
}

/// User record (no password hash exposed). Returned by list_users and
/// the various admin CRUD commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: UserRole,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

// ── Hashing helpers ────────────────────────────────────────────────────────

pub fn hash_password(password: &str) -> Result<String, String> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "Password must be at least {} characters",
            MIN_PASSWORD_LEN
        ));
    }
    // Cheap low-entropy guards against the most predictable choices. Full
    // breached-password (HIBP) integration is a future improvement.
    //
    // 1. All-lowercase-alphanumeric must mix letters and digits.
    let lower = password.to_lowercase();
    if lower == password && password.chars().all(|c| c.is_ascii_alphanumeric()) {
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
        if !has_digit || !has_letter {
            return Err("Password must include both letters and digits".to_string());
        }
    }
    // 2. Require a minimum number of distinct characters. Rule 1 is bypassed by
    //    a single uppercase letter (e.g. "Aaaaaaaaaaaa"); this catches those.
    let distinct = password.chars().collect::<std::collections::HashSet<_>>().len();
    if distinct < MIN_DISTINCT_CHARS {
        return Err(format!(
            "Password must use at least {} different characters",
            MIN_DISTINCT_CHARS
        ));
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {}", e))?
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_passwords() {
        assert!(hash_password("short").is_err());
        // 11 chars — one below MIN_PASSWORD_LEN
        assert!(hash_password("eleven_char").is_err());
    }

    #[test]
    fn rejects_all_letters_or_all_digits() {
        // 12 lowercase letters with no digits → rejected
        assert!(hash_password("abcdefghijkl").is_err());
        // 12 digits with no letters → rejected
        assert!(hash_password("123456789012").is_err());
    }

    #[test]
    fn accepts_strong_passwords() {
        assert!(hash_password("correcthorsebattery9").is_ok());
        assert!(hash_password("PassPhrase2026!").is_ok());
        // Mixed-case alphanumeric is fine even without digits because the
        // letter/digit rule only kicks in for lowercase-alphanumeric strings,
        // and these have plenty of distinct characters.
        assert!(hash_password("PassPhraseRSA").is_ok());
    }

    #[test]
    fn rejects_low_diversity_even_with_uppercase() {
        // Adding a single uppercase char used to bypass the composition check.
        assert!(hash_password("Aaaaaaaaaaaa").is_err());
        assert!(hash_password("AAAAAAAAAAAA").is_err());
        // Distinct-char floor also catches short-alphabet repeats.
        assert!(hash_password("Ab1Ab1Ab1Ab1").is_err());
    }

    #[test]
    fn lockout_active_during_lockout_duration() {
        assert_eq!(
            evaluate_lockout(MAX_LOGIN_FAILURES, Duration::from_secs(0)),
            LockoutCheck::Locked
        );
        assert_eq!(
            evaluate_lockout(MAX_LOGIN_FAILURES, LOCKOUT_DURATION - Duration::from_secs(1)),
            LockoutCheck::Locked
        );
    }

    #[test]
    fn lockout_resets_once_duration_served_no_unthrottled_gap() {
        // The regression: at 61s the old code left the counter >= MAX but
        // stopped enforcing the lockout, giving ~240s of unlimited guesses.
        // Now the served lockout resets to a fresh (re-lockable) budget.
        assert_eq!(
            evaluate_lockout(MAX_LOGIN_FAILURES, LOCKOUT_DURATION + Duration::from_secs(1)),
            LockoutCheck::Reset
        );
        // Anywhere between LOCKOUT_DURATION and LOCKOUT_WINDOW must NOT proceed
        // with the counter intact — it must reset, never silently allow.
        assert_ne!(
            evaluate_lockout(MAX_LOGIN_FAILURES, Duration::from_secs(200)),
            LockoutCheck::Proceed
        );
    }

    #[test]
    fn sub_threshold_counter_proceeds_then_ages_out() {
        assert_eq!(
            evaluate_lockout(MAX_LOGIN_FAILURES - 1, Duration::from_secs(5)),
            LockoutCheck::Proceed
        );
        assert_eq!(
            evaluate_lockout(3, LOCKOUT_WINDOW + Duration::from_secs(1)),
            LockoutCheck::Reset
        );
    }
}

/// A precomputed Argon2 hash used only to equalize login timing when the
/// username doesn't exist: without it, an unknown user returns immediately
/// while a known user pays the (expensive) Argon2 verify, letting an attacker
/// enumerate valid usernames by response time. Computed once, lazily.
fn dummy_password_hash() -> &'static str {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DUMMY
        .get_or_init(|| {
            hash_password("timing-equalizer-not-a-real-password").unwrap_or_default()
        })
        .as_str()
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let parsed = match PasswordHash::new(stored_hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Read the active session as a clone. Used by other modules
/// (e.g. editor.rs for audit-log attribution).
pub fn current_session(state: &AppState) -> Option<UserSession> {
    state.current_user.lock().ok().and_then(|g| g.clone())
}

// ── Commands ───────────────────────────────────────────────────────────────

/// True when no users exist yet — the frontend should show the
/// "Create your first admin" form instead of the login screen.
#[tauri::command]
pub fn is_setup_required(state: State<AppState>) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count == 0)
}

/// Create the first admin. Refuses if any user already exists (defensive
/// — only relevant on bootstrap).
#[tauri::command]
pub fn create_first_admin(
    username: String,
    password: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<UserSession, String> {
    if username.trim().is_empty() {
        return Err("Username is required".to_string());
    }
    let hash = hash_password(&password)?;

    let session = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count > 0 {
            return Err("Setup already complete — use login instead".to_string());
        }
        db.execute(
            "INSERT INTO users (username, password_hash, role, last_login_at)
             VALUES (?1, ?2, 'admin', datetime('now'))",
            rusqlite::params![username.trim(), hash],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "Username already taken".to_string()
            } else {
                e.to_string()
            }
        })?;
        let user_id = db.last_insert_rowid();
        UserSession {
            user_id,
            username: username.trim().to_string(),
            role: UserRole::Admin,
            login_at_ms: now_ms(),
        }
    };

    set_current_session(&state, Some(session.clone()));
    let _ = app.emit("auth:changed", &Some(session.clone()));
    Ok(session)
}

#[tauri::command]
pub fn login(
    username: String,
    password: String,
    app: AppHandle,
    state: State<AppState>,
) -> Result<UserSession, String> {
    let username_key = username.trim().to_lowercase();

    // Lockout check (Plan 11). Reject early if the user has burned through
    // their failed-login budget within the lockout window.
    {
        let mut attempts = state
            .login_attempts
            .lock()
            .map_err(|e| e.to_string())?;
        if let Some((count, first_at)) = attempts.get(&username_key).copied() {
            match evaluate_lockout(count, first_at.elapsed()) {
                LockoutCheck::Locked => {
                    return Err(
                        "Too many failed attempts. Try again in a minute.".to_string(),
                    );
                }
                LockoutCheck::Reset => {
                    attempts.remove(&username_key);
                }
                LockoutCheck::Proceed => {}
            }
        }
    }

    let session_result: Result<UserSession, String> = (|| {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let Some((user_id, db_username, password_hash, role_str)) = db
            .query_row(
                "SELECT id, username, password_hash, role FROM users
                 WHERE LOWER(username) = LOWER(?1)",
                rusqlite::params![username.trim()],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .ok()
        else {
            // Run a verify against a dummy hash so an unknown username takes
            // about as long as a wrong password — no user-enumeration by timing.
            // Same generic error for unknown user vs wrong password.
            let _ = verify_password(&password, dummy_password_hash());
            return Err("Invalid username or password".to_string());
        };
        if !verify_password(&password, &password_hash) {
            return Err("Invalid username or password".to_string());
        }
        let role = UserRole::from_db_str(&role_str)
            .ok_or_else(|| format!("Unknown role in database: {}", role_str))?;

        // Touch last_login_at
        let _ = db.execute(
            "UPDATE users SET last_login_at = datetime('now') WHERE id = ?1",
            rusqlite::params![user_id],
        );

        // Use the canonical username from the DB so casing matches what
        // was originally registered, regardless of how the user typed it.
        Ok(UserSession {
            user_id,
            username: db_username,
            role,
            login_at_ms: now_ms(),
        })
    })();

    let session = match session_result {
        Ok(s) => s,
        Err(e) => {
            // Bump the failed-attempt counter.
            if let Ok(mut attempts) = state.login_attempts.lock() {
                let entry = attempts
                    .entry(username_key.clone())
                    .or_insert((0, Instant::now()));
                if entry.1.elapsed() > LOCKOUT_WINDOW {
                    *entry = (0, Instant::now());
                }
                entry.0 += 1;
            }
            return Err(e);
        }
    };

    // Successful login — clear the counter for this username.
    if let Ok(mut attempts) = state.login_attempts.lock() {
        attempts.remove(&username_key);
    }

    set_current_session(&state, Some(session.clone()));
    let _ = app.emit("auth:changed", &Some(session.clone()));
    Ok(session)
}

#[tauri::command]
pub fn logout(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    set_current_session(&state, None);
    let _ = app.emit("auth:changed", &None::<UserSession>);
    Ok(())
}

#[tauri::command]
pub fn get_current_user(state: State<AppState>) -> Option<UserSession> {
    current_session(&state)
}

// ── Internal helpers ───────────────────────────────────────────────────────

fn set_current_session(state: &State<AppState>, session: Option<UserSession>) {
    if let Ok(mut g) = state.current_user.lock() {
        *g = session;
    }
}

/// Helper used by user_management.rs to gate admin-only commands.
pub fn require_admin(state: &State<AppState>) -> Result<UserSession, String> {
    let session = current_session(state).ok_or_else(|| "Not logged in".to_string())?;
    if session.role != UserRole::Admin {
        return Err("Admin access required".to_string());
    }
    Ok(session)
}

/// Helper used to gate commands that need any logged-in user.
/// Use over `require_admin` when an editor is also allowed (e.g. metadata
/// editing, share creation, order fulfillment).
pub fn require_session(state: &State<AppState>) -> Result<UserSession, String> {
    current_session(state).ok_or_else(|| "Not logged in".to_string())
}
