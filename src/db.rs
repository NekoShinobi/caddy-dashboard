use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::sync::Arc;

pub const LOGS: TableDefinition<u64, &str> = TableDefinition::new("logs");
pub const META: TableDefinition<u64, u64> = TableDefinition::new("meta");
pub const USERS: TableDefinition<&str, &str> = TableDefinition::new("users");
pub const SESSIONS: TableDefinition<&str, &str> = TableDefinition::new("sessions");
pub const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");
/// Maps session_token → OIDC id_token (only populated for OIDC-authenticated sessions).
pub const OIDC_TOKENS: TableDefinition<&str, &str> = TableDefinition::new("oidc_tokens");

pub const META_LAST_POS: u64 = 0;
pub const META_LAST_INODE: u64 = 1;
pub const META_NEXT_ID: u64 = 2;

pub fn open(data_dir: &str) -> Arc<Database> {
    std::fs::create_dir_all(data_dir).expect("failed to create data dir");
    let path = format!("{data_dir}/caddy.db");
    let db = Database::create(&path).expect("failed to open database");
    // Create tables so read transactions never fail with "table does not exist"
    let wtxn = db.begin_write().expect("failed to init tables");
    wtxn.open_table(LOGS).expect("failed to open logs table");
    wtxn.open_table(META).expect("failed to open meta table");
    wtxn.open_table(USERS).expect("failed to open users table");
    wtxn.open_table(SESSIONS).expect("failed to open sessions table");
    wtxn.open_table(SETTINGS).expect("failed to open settings table");
    wtxn.open_table(OIDC_TOKENS).expect("failed to open oidc_tokens table");
    wtxn.commit().expect("failed to commit table init");
    Arc::new(db)
}

/// Wipe all users, sessions, and OIDC tokens. Used by USER_DATABASE_RESET on startup.
pub fn reset_users(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let wtxn = db.begin_write()?;
    {
        let mut users = wtxn.open_table(USERS)?;
        let keys: Vec<String> = users.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
        for k in &keys { users.remove(k.as_str())?; }
        let mut sessions = wtxn.open_table(SESSIONS)?;
        let stokens: Vec<String> = sessions.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
        for t in &stokens { sessions.remove(t.as_str())?; }
        let mut oidc = wtxn.open_table(OIDC_TOKENS)?;
        let otokens: Vec<String> = oidc.iter()?.filter_map(|r| r.ok().map(|(k, _)| k.value().to_string())).collect();
        for t in &otokens { oidc.remove(t.as_str())?; }
    }
    wtxn.commit()?;
    Ok(())
}

pub fn purge_old(db: &Database, cutoff_ts: f64) -> usize {
    (|| -> Result<usize, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(LOGS)?;
        let keys_to_delete: Vec<u64> = table
            .iter()?
            .filter_map(|r| {
                let (k, v) = r.ok()?;
                let entry: crate::log_parser::LogEntry = serde_json::from_str(v.value()).ok()?;
                if entry.ts < cutoff_ts { Some(k.value()) } else { None }
            })
            .collect();
        drop(table);
        drop(rtxn);

        let count = keys_to_delete.len();
        if count == 0 {
            return Ok(0);
        }
        let wtxn = db.begin_write()?;
        {
            let mut table = wtxn.open_table(LOGS)?;
            for key in keys_to_delete {
                table.remove(key)?;
            }
        }
        wtxn.commit()?;
        Ok(count)
    })()
    .inspect_err(|e| log::error!("purge_old: {e}"))
    .unwrap_or(0)
}

// ── Users ────────────────────────────────────────────────────────────────────

pub fn user_count(db: &Database) -> usize {
    (|| -> Result<usize, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(USERS)?;
        Ok(table.len()? as usize)
    })()
    .inspect_err(|e| log::error!("user_count: {e}"))
    .unwrap_or(0)
}

pub fn get_user(db: &Database, username: &str) -> Option<crate::auth::User> {
    let rtxn = db.begin_read()
        .inspect_err(|e| log::error!("get_user({username}): begin_read: {e}")).ok()?;
    let table = rtxn.open_table(USERS)
        .inspect_err(|e| log::error!("get_user({username}): open_table: {e}")).ok()?;
    let val = table.get(username)
        .inspect_err(|e| log::error!("get_user({username}): table.get: {e}")).ok()??;
    serde_json::from_str(val.value())
        .inspect_err(|e| log::error!("get_user({username}): deserialize: {e}")).ok()
}

pub fn get_user_by_email(db: &Database, email: &str) -> Option<crate::auth::User> {
    let rtxn = db.begin_read()
        .inspect_err(|e| log::error!("get_user_by_email: begin_read: {e}")).ok()?;
    let table = rtxn.open_table(USERS)
        .inspect_err(|e| log::error!("get_user_by_email: open_table: {e}")).ok()?;
    for result in table.iter().ok()? {
        let (_, v) = result.ok()?;
        if let Ok(u) = serde_json::from_str::<crate::auth::User>(v.value()) {
            if u.email.eq_ignore_ascii_case(email) {
                return Some(u);
            }
        }
    }
    None
}

pub fn list_users(db: &Database) -> Vec<crate::auth::User> {
    (|| -> Result<Vec<_>, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(USERS)?;
        let mut users = Vec::new();
        for result in table.iter()? {
            let (_, v) = result?;
            match serde_json::from_str::<crate::auth::User>(v.value()) {
                Ok(u) => users.push(u),
                Err(e) => log::error!("list_users: deserialize row: {e}"),
            }
        }
        Ok(users)
    })()
    .inspect_err(|e| log::error!("list_users: {e}"))
    .unwrap_or_default()
}

pub fn create_user(db: &Database, user: &crate::auth::User) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(user)?;
        let wtxn = db.begin_write()?;
        {
            let mut table = wtxn.open_table(USERS)?;
            table.insert(user.username.as_str(), json.as_str())?;
        }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("create_user({}): {e}", user.username))
    .is_ok()
}

pub fn delete_user(db: &Database, username: &str) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        {
            let mut table = wtxn.open_table(USERS)?;
            table.remove(username)?;
        }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("delete_user({username}): {e}"))
    .is_ok()
}

/// Update a user record. If `updated.username != old_username` the old key is removed.
pub fn update_user(db: &Database, old_username: &str, updated: &crate::auth::User) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(updated)?;
        let wtxn = db.begin_write()?;
        {
            let mut table = wtxn.open_table(USERS)?;
            if old_username != updated.username.as_str() {
                table.remove(old_username)?;
            }
            table.insert(updated.username.as_str(), json.as_str())?;
        }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("update_user({old_username}): {e}"))
    .is_ok()
}

pub fn update_password(db: &Database, username: &str, new_hash: &str) -> bool {
    let Some(mut user) = get_user(db, username) else {
        log::error!("update_password({username}): user not found");
        return false;
    };
    user.password_hash = new_hash.to_string();
    create_user(db, &user)
}

// ── Sessions ─────────────────────────────────────────────────────────────────

pub fn create_session(db: &Database, token: &str, username: &str) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        { let mut t = wtxn.open_table(SESSIONS)?; t.insert(token, username)?; }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("create_session({username}): {e}"))
    .is_ok()
}

pub fn get_session(db: &Database, token: &str) -> Option<String> {
    let rtxn = db.begin_read()
        .inspect_err(|e| log::error!("get_session: begin_read: {e}")).ok()?;
    let table = rtxn.open_table(SESSIONS)
        .inspect_err(|e| log::error!("get_session: open_table: {e}")).ok()?;
    let val = table.get(token)
        .inspect_err(|e| log::error!("get_session: table.get: {e}")).ok()??;
    Some(val.value().to_string())
}

pub fn delete_session(db: &Database, token: &str) {
    if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        {
            let mut t = wtxn.open_table(SESSIONS)?; t.remove(token)?;
            let mut ot = wtxn.open_table(OIDC_TOKENS)?; ot.remove(token)?;
        }
        wtxn.commit()?;
        Ok(())
    })() {
        log::error!("delete_session: {e}");
    }
}

/// Delete all sessions (and OIDC tokens) belonging to `username`.
pub fn delete_user_sessions(db: &Database, username: &str) {
    if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(SESSIONS)?;
        let tokens: Vec<String> = table
            .iter()?
            .filter_map(|r| {
                let (k, v) = r.ok()?;
                if v.value() == username { Some(k.value().to_string()) } else { None }
            })
            .collect();
        drop(table);
        drop(rtxn);
        if tokens.is_empty() { return Ok(()); }
        let wtxn = db.begin_write()?;
        {
            let mut t = wtxn.open_table(SESSIONS)?;
            let mut ot = wtxn.open_table(OIDC_TOKENS)?;
            for tok in &tokens {
                t.remove(tok.as_str())?;
                ot.remove(tok.as_str())?;
            }
        }
        wtxn.commit()?;
        Ok(())
    })() {
        log::error!("delete_user_sessions({username}): {e}");
    }
}

// ── OIDC token storage ───────────────────────────────────────────────────────

pub fn store_oidc_token(db: &Database, session_token: &str, id_token: &str) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        { let mut t = wtxn.open_table(OIDC_TOKENS)?; t.insert(session_token, id_token)?; }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("store_oidc_token: {e}"))
    .is_ok()
}

pub fn get_oidc_token(db: &Database, session_token: &str) -> Option<String> {
    let rtxn = db.begin_read()
        .inspect_err(|e| log::error!("get_oidc_token: begin_read: {e}")).ok()?;
    let table = rtxn.open_table(OIDC_TOKENS)
        .inspect_err(|e| log::error!("get_oidc_token: open_table: {e}")).ok()?;
    let val = table.get(session_token)
        .inspect_err(|e| log::error!("get_oidc_token: table.get: {e}")).ok()??;
    Some(val.value().to_string())
}

// ── Site settings ────────────────────────────────────────────────────────────

pub fn get_setting(db: &Database, key: &str) -> Option<String> {
    let rtxn = db.begin_read()
        .inspect_err(|e| log::error!("get_setting({key}): begin_read: {e}")).ok()?;
    let table = rtxn.open_table(SETTINGS)
        .inspect_err(|e| log::error!("get_setting({key}): open_table: {e}")).ok()?;
    let val = table.get(key)
        .inspect_err(|e| log::error!("get_setting({key}): table.get: {e}")).ok()??;
    Some(val.value().to_string())
}

pub fn set_setting(db: &Database, key: &str, value: &str) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        { let mut t = wtxn.open_table(SETTINGS)?; t.insert(key, value)?; }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("set_setting({key}): {e}"))
    .is_ok()
}

// ── Log entries ───────────────────────────────────────────────────────────────

pub fn load_entries(db: &Database) -> Result<Vec<crate::log_parser::LogEntry>, String> {
    (|| -> Result<Vec<_>, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(LOGS)?;
        let mut entries = Vec::new();
        for result in table.iter()? {
            let (_, v) = result?;
            match serde_json::from_str(v.value()) {
                Ok(entry) => entries.push(entry),
                Err(e) => log::error!("load_entries: deserialize row: {e}"),
            }
        }
        Ok(entries)
    })()
    .map_err(|e| {
        log::error!("load_entries: {e}");
        "Database error".to_string()
    })
}
