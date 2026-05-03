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
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

const MIN_PASSWORD_LEN: usize = 6;

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
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {}", e))?
        .to_string())
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
    let session = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let row = db
            .query_row(
                "SELECT id, password_hash, role FROM users WHERE username = ?1",
                rusqlite::params![username.trim()],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .ok();

        let Some((user_id, password_hash, role_str)) = row else {
            // Same generic error for unknown user vs wrong password —
            // standard practice to avoid username enumeration.
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

        UserSession {
            user_id,
            username: username.trim().to_string(),
            role,
            login_at_ms: now_ms(),
        }
    };

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
