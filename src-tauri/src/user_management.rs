// Admin user CRUD (Plan 10).
//
// Every command except update_user_password is admin-only. The password
// change command allows any logged-in user to change their OWN password
// without admin rights — admins can change anyone's.
//
// Two safety checks prevent the database from getting into an unusable
// state:
//   1. Cannot delete yourself (must be done by another admin).
//   2. Cannot delete or downgrade a user if it would leave zero admins.

use crate::auth::{
    current_session, hash_password, require_admin, User, UserRole,
};
use crate::db::AppState;
use rusqlite::params;
use tauri::State;

// ── Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_users(state: State<AppState>) -> Result<Vec<User>, String> {
    require_admin(&state)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, username, role, created_at, last_login_at
             FROM users ORDER BY username COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let users: Vec<User> = rows
        .filter_map(|res| res.ok())
        .filter_map(|(id, username, role_str, created_at, last_login_at)| {
            UserRole::from_db_str(&role_str).map(|role| User {
                id,
                username,
                role,
                created_at,
                last_login_at,
            })
        })
        .collect();
    Ok(users)
}

#[tauri::command]
pub fn create_user(
    username: String,
    password: String,
    role: UserRole,
    state: State<AppState>,
) -> Result<User, String> {
    require_admin(&state)?;
    if username.trim().is_empty() {
        return Err("Username is required".to_string());
    }
    let hash = hash_password(&password)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3)",
        params![username.trim(), hash, role.as_str()],
    )
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("idx_users_username_nocase") {
            "Username already taken (matches existing user, ignoring case)".to_string()
        } else if msg.contains("UNIQUE") {
            "Username already taken".to_string()
        } else {
            msg
        }
    })?;
    let id = db.last_insert_rowid();
    Ok(User {
        id,
        username: username.trim().to_string(),
        role,
        created_at: db
            .query_row(
                "SELECT created_at FROM users WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or_default(),
        last_login_at: None,
    })
}

#[tauri::command]
pub fn update_user_role(
    user_id: i64,
    role: UserRole,
    state: State<AppState>,
) -> Result<(), String> {
    let _admin_session = require_admin(&state)?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;

    // Look up current role inside the transaction so the count check below
    // sees a consistent snapshot.
    let current_role = {
        let (_username, role_str) = tx
            .query_row(
                "SELECT username, role FROM users WHERE id = ?1",
                params![user_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|_| "User not found".to_string())?;
        UserRole::from_db_str(&role_str).ok_or_else(|| "Unknown role".to_string())?
    };

    // If demoting an admin, ensure at least one other admin remains.
    if current_role == UserRole::Admin && role != UserRole::Admin {
        let admins: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role = 'admin'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if admins <= 1 {
            return Err(
                "Can't demote the last remaining admin. Promote another user first."
                    .to_string(),
            );
        }
    }

    tx.execute(
        "UPDATE users SET role = ?1 WHERE id = ?2",
        params![role.as_str(), user_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_user_password(
    user_id: i64,
    new_password: String,
    state: State<AppState>,
) -> Result<(), String> {
    let session = current_session(&state).ok_or_else(|| "Not logged in".to_string())?;
    // Anyone can change their own password; only admins can change others'.
    if session.user_id != user_id && session.role != UserRole::Admin {
        return Err("You can only change your own password".to_string());
    }
    let hash = hash_password(&new_password)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE users SET password_hash = ?1 WHERE id = ?2",
        params![hash, user_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_user(user_id: i64, state: State<AppState>) -> Result<(), String> {
    let admin_session = require_admin(&state)?;
    if admin_session.user_id == user_id {
        return Err("You can't delete your own account".to_string());
    }
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;

    let target_role = {
        let role_str: String = tx
            .query_row(
                "SELECT role FROM users WHERE id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .map_err(|_| "User not found".to_string())?;
        UserRole::from_db_str(&role_str).ok_or_else(|| "Unknown role".to_string())?
    };

    if target_role == UserRole::Admin {
        let admins: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role = 'admin'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if admins <= 1 {
            return Err("Can't delete the last admin".to_string());
        }
    }

    tx.execute("DELETE FROM users WHERE id = ?1", params![user_id])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
