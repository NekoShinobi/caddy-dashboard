use crate::analytics::{self, RangeSegment, Resolution, Rollup};
use crate::log_parser::LogEntry;
use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, Table, TableDefinition,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

pub const LOGS: TableDefinition<u64, &str> = TableDefinition::new("logs");
pub const LOG_TIMES: TableDefinition<(u64, u64), ()> = TableDefinition::new("log_times");
pub const STATUS_LOGS: TableDefinition<(u16, u64), ()> = TableDefinition::new("status_logs");
pub const METHOD_LOGS: TableDefinition<(&str, u64), ()> = TableDefinition::new("method_logs");
pub const ROLLUPS_MINUTE: TableDefinition<u64, &str> = TableDefinition::new("rollups_minute");
pub const ROLLUPS_HOUR: TableDefinition<u64, &str> = TableDefinition::new("rollups_hour");
pub const ROLLUPS_DAY: TableDefinition<u64, &str> = TableDefinition::new("rollups_day");
pub const META: TableDefinition<u64, u64> = TableDefinition::new("meta");
pub const USERS: TableDefinition<&str, &str> = TableDefinition::new("users");
pub const SESSIONS: TableDefinition<&str, &str> = TableDefinition::new("sessions");
pub const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");
/// Maps session_token → OIDC id_token (only populated for OIDC-authenticated sessions).
pub const OIDC_TOKENS: TableDefinition<&str, &str> = TableDefinition::new("oidc_tokens");

pub const META_LAST_POS: u64 = 0;
pub const META_LAST_INODE: u64 = 1;
pub const META_NEXT_ID: u64 = 2;
pub const META_ANALYTICS_VERSION: u64 = 3;
pub const META_DATA_REVISION: u64 = 4;
pub const META_COMPACTED_HOUR: u64 = 5;
pub const META_COMPACTED_DAY: u64 = 6;
const ANALYTICS_VERSION: u64 = 1;
const MIGRATION_BATCH_SIZE: usize = 10_000;

pub fn open(data_dir: &str) -> Arc<Database> {
    std::fs::create_dir_all(data_dir).expect("failed to create data dir");
    let path = format!("{data_dir}/caddy.db");
    let db = Database::create(&path).expect("failed to open database");
    // Create tables so read transactions never fail with "table does not exist"
    let wtxn = db.begin_write().expect("failed to init tables");
    wtxn.open_table(LOGS).expect("failed to open logs table");
    wtxn.open_table(LOG_TIMES)
        .expect("failed to open log_times table");
    wtxn.open_table(STATUS_LOGS)
        .expect("failed to open status_logs table");
    wtxn.open_table(METHOD_LOGS)
        .expect("failed to open method_logs table");
    wtxn.open_table(ROLLUPS_MINUTE)
        .expect("failed to open minute rollups table");
    wtxn.open_table(ROLLUPS_HOUR)
        .expect("failed to open hour rollups table");
    wtxn.open_table(ROLLUPS_DAY)
        .expect("failed to open day rollups table");
    wtxn.open_table(META).expect("failed to open meta table");
    wtxn.open_table(USERS).expect("failed to open users table");
    wtxn.open_table(SESSIONS)
        .expect("failed to open sessions table");
    wtxn.open_table(SETTINGS)
        .expect("failed to open settings table");
    wtxn.open_table(OIDC_TOKENS)
        .expect("failed to open oidc_tokens table");
    wtxn.commit().expect("failed to commit table init");
    migrate_analytics(&db).expect("failed to build analytics indexes");
    Arc::new(db)
}

/// Wipe all users, sessions, and OIDC tokens. Used by USER_DATABASE_RESET on startup.
pub fn reset_users(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let wtxn = db.begin_write()?;
    {
        let mut users = wtxn.open_table(USERS)?;
        let keys: Vec<String> = users
            .iter()?
            .filter_map(|r| r.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for k in &keys {
            users.remove(k.as_str())?;
        }
        let mut sessions = wtxn.open_table(SESSIONS)?;
        let stokens: Vec<String> = sessions
            .iter()?
            .filter_map(|r| r.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for t in &stokens {
            sessions.remove(t.as_str())?;
        }
        let mut oidc = wtxn.open_table(OIDC_TOKENS)?;
        let otokens: Vec<String> = oidc
            .iter()?
            .filter_map(|r| r.ok().map(|(k, _)| k.value().to_string()))
            .collect();
        for t in &otokens {
            oidc.remove(t.as_str())?;
        }
    }
    wtxn.commit()?;
    Ok(())
}

pub fn purge_old(db: &Database, cutoff_ts: f64) -> usize {
    (|| -> Result<usize, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let times = rtxn.open_table(LOG_TIMES)?;
        let logs = rtxn.open_table(LOGS)?;
        let mut to_delete = Vec::new();
        for result in times.range(..=(timestamp_millis(cutoff_ts), u64::MAX))? {
            let (time_key, _) = result?;
            let (millis, id) = time_key.value();
            let Some(value) = logs.get(id)? else { continue };
            let entry: LogEntry = serde_json::from_str(value.value())?;
            if entry.ts < cutoff_ts {
                to_delete.push(((millis, id), entry.status, entry.request.method));
            }
        }
        drop(logs);
        drop(times);
        drop(rtxn);

        let count = to_delete.len();
        if count == 0 {
            return Ok(0);
        }
        let wtxn = db.begin_write()?;
        {
            let mut logs = wtxn.open_table(LOGS)?;
            let mut times = wtxn.open_table(LOG_TIMES)?;
            let mut statuses = wtxn.open_table(STATUS_LOGS)?;
            let mut methods = wtxn.open_table(METHOD_LOGS)?;
            for ((time_key, id), status, method) in &to_delete {
                logs.remove(*id)?;
                times.remove((*time_key, *id))?;
                statuses.remove((*status, *id))?;
                methods.remove((method.to_ascii_uppercase().as_str(), *id))?;
            }

            let minute_boundary = analytics::bucket_start(cutoff_ts, analytics::MINUTE_SECS);
            let hour_boundary = analytics::bucket_start(cutoff_ts, analytics::HOUR_SECS);
            let day_boundary = analytics::bucket_start(cutoff_ts, analytics::DAY_SECS);
            let mut minute = wtxn.open_table(ROLLUPS_MINUTE)?;
            minute.retain(|bucket, _| bucket >= minute_boundary)?;
            minute.remove(minute_boundary)?;
            let mut hour = wtxn.open_table(ROLLUPS_HOUR)?;
            hour.retain(|bucket, _| bucket >= hour_boundary)?;
            hour.remove(hour_boundary)?;
            let mut day = wtxn.open_table(ROLLUPS_DAY)?;
            day.retain(|bucket, _| bucket >= day_boundary)?;
            day.remove(day_boundary)?;
            let mut meta = wtxn.open_table(META)?;
            let revision = meta
                .get(META_DATA_REVISION)?
                .map(|value| value.value())
                .unwrap_or(0);
            meta.insert(META_DATA_REVISION, revision.wrapping_add(1))?;
        }
        wtxn.commit()?;

        rebuild_retention_boundaries(db, cutoff_ts)?;
        Ok(count)
    })()
    .inspect_err(|e| log::error!("purge_old: {e}"))
    .unwrap_or(0)
}

fn rebuild_retention_boundaries(
    db: &Database,
    cutoff_ts: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let day_boundary = analytics::bucket_start(cutoff_ts, analytics::DAY_SECS);
    let day_end = day_boundary + analytics::DAY_SECS;
    let entries = load_entries_between(db, cutoff_ts, day_end as f64)?;
    let minute_boundary = analytics::bucket_start(cutoff_ts, analytics::MINUTE_SECS);
    let hour_boundary = analytics::bucket_start(cutoff_ts, analytics::HOUR_SECS);

    let minute =
        analytics::build_updates(&entries, analytics::MINUTE_SECS).remove(&minute_boundary);
    let hour = analytics::build_updates(&entries, analytics::HOUR_SECS).remove(&hour_boundary);
    let day = analytics::build_updates(&entries, analytics::DAY_SECS).remove(&day_boundary);
    let wtxn = db.begin_write()?;
    {
        if let Some(rollup) = minute {
            let json = serde_json::to_string(&rollup)?;
            wtxn.open_table(ROLLUPS_MINUTE)?
                .insert(minute_boundary, json.as_str())?;
        }
        if let Some(rollup) = hour {
            let json = serde_json::to_string(&rollup)?;
            wtxn.open_table(ROLLUPS_HOUR)?
                .insert(hour_boundary, json.as_str())?;
        }
        if let Some(rollup) = day {
            let json = serde_json::to_string(&rollup)?;
            wtxn.open_table(ROLLUPS_DAY)?
                .insert(day_boundary, json.as_str())?;
        }
    }
    wtxn.commit()?;
    Ok(())
}

fn load_entries_between(
    db: &Database,
    start: f64,
    end: f64,
) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
    let rtxn = db.begin_read()?;
    let times = rtxn.open_table(LOG_TIMES)?;
    let logs = rtxn.open_table(LOGS)?;
    let mut entries = Vec::new();
    for item in times.range((timestamp_millis(start), 0)..=(timestamp_millis(end), u64::MAX))? {
        let (key, _) = item?;
        let (_, id) = key.value();
        let Some(value) = logs.get(id)? else { continue };
        let entry: LogEntry = serde_json::from_str(value.value())?;
        if entry.ts >= start && entry.ts < end {
            entries.push(entry);
        }
    }
    Ok(entries)
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
    let rtxn = db
        .begin_read()
        .inspect_err(|e| log::error!("get_user({username}): begin_read: {e}"))
        .ok()?;
    let table = rtxn
        .open_table(USERS)
        .inspect_err(|e| log::error!("get_user({username}): open_table: {e}"))
        .ok()?;
    let val = table
        .get(username)
        .inspect_err(|e| log::error!("get_user({username}): table.get: {e}"))
        .ok()??;
    serde_json::from_str(val.value())
        .inspect_err(|e| log::error!("get_user({username}): deserialize: {e}"))
        .ok()
}

pub fn get_user_by_email(db: &Database, email: &str) -> Option<crate::auth::User> {
    let rtxn = db
        .begin_read()
        .inspect_err(|e| log::error!("get_user_by_email: begin_read: {e}"))
        .ok()?;
    let table = rtxn
        .open_table(USERS)
        .inspect_err(|e| log::error!("get_user_by_email: open_table: {e}"))
        .ok()?;
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
        {
            let mut t = wtxn.open_table(SESSIONS)?;
            t.insert(token, username)?;
        }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("create_session({username}): {e}"))
    .is_ok()
}

pub fn get_session(db: &Database, token: &str) -> Option<String> {
    let rtxn = db
        .begin_read()
        .inspect_err(|e| log::error!("get_session: begin_read: {e}"))
        .ok()?;
    let table = rtxn
        .open_table(SESSIONS)
        .inspect_err(|e| log::error!("get_session: open_table: {e}"))
        .ok()?;
    let val = table
        .get(token)
        .inspect_err(|e| log::error!("get_session: table.get: {e}"))
        .ok()??;
    Some(val.value().to_string())
}

pub fn delete_session(db: &Database, token: &str) {
    if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        {
            let mut t = wtxn.open_table(SESSIONS)?;
            t.remove(token)?;
            let mut ot = wtxn.open_table(OIDC_TOKENS)?;
            ot.remove(token)?;
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
                if v.value() == username {
                    Some(k.value().to_string())
                } else {
                    None
                }
            })
            .collect();
        drop(table);
        drop(rtxn);
        if tokens.is_empty() {
            return Ok(());
        }
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
        {
            let mut t = wtxn.open_table(OIDC_TOKENS)?;
            t.insert(session_token, id_token)?;
        }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("store_oidc_token: {e}"))
    .is_ok()
}

pub fn get_oidc_token(db: &Database, session_token: &str) -> Option<String> {
    let rtxn = db
        .begin_read()
        .inspect_err(|e| log::error!("get_oidc_token: begin_read: {e}"))
        .ok()?;
    let table = rtxn
        .open_table(OIDC_TOKENS)
        .inspect_err(|e| log::error!("get_oidc_token: open_table: {e}"))
        .ok()?;
    let val = table
        .get(session_token)
        .inspect_err(|e| log::error!("get_oidc_token: table.get: {e}"))
        .ok()??;
    Some(val.value().to_string())
}

// ── Site settings ────────────────────────────────────────────────────────────

pub fn get_setting(db: &Database, key: &str) -> Option<String> {
    let rtxn = db
        .begin_read()
        .inspect_err(|e| log::error!("get_setting({key}): begin_read: {e}"))
        .ok()?;
    let table = rtxn
        .open_table(SETTINGS)
        .inspect_err(|e| log::error!("get_setting({key}): open_table: {e}"))
        .ok()?;
    let val = table
        .get(key)
        .inspect_err(|e| log::error!("get_setting({key}): table.get: {e}"))
        .ok()??;
    Some(val.value().to_string())
}

pub fn set_setting(db: &Database, key: &str, value: &str) -> bool {
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let wtxn = db.begin_write()?;
        {
            let mut t = wtxn.open_table(SETTINGS)?;
            t.insert(key, value)?;
        }
        wtxn.commit()?;
        Ok(())
    })()
    .inspect_err(|e| log::error!("set_setting({key}): {e}"))
    .is_ok()
}

// ── Log entries ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum LogScan {
    All,
    Status(u16),
    Method(String),
}

pub struct LogPage {
    pub entries: Vec<LogEntry>,
    pub next_cursor: Option<u64>,
    pub has_more: bool,
    pub total: Option<u64>,
    pub scanned: usize,
}

fn timestamp_millis(ts: f64) -> u64 {
    if ts.is_finite() && ts > 0.0 {
        (ts * 1_000.0).floor().min(u64::MAX as f64) as u64
    } else {
        0
    }
}

fn write_rollups(
    table: &mut Table<'_, u64, &str>,
    updates: HashMap<u64, Rollup>,
) -> Result<(), Box<dyn std::error::Error>> {
    for (bucket, update) in updates {
        let mut combined = table
            .get(bucket)?
            .map(|value| serde_json::from_str::<Rollup>(value.value()))
            .transpose()?
            .unwrap_or_default();
        combined.merge(&update);
        combined.finish_bucket();
        let json = serde_json::to_string(&combined)?;
        table.insert(bucket, json.as_str())?;
    }
    Ok(())
}

fn migrate_analytics(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let current = {
        let rtxn = db.begin_read()?;
        let meta = rtxn.open_table(META)?;
        meta.get(META_ANALYTICS_VERSION)?
            .map(|value| value.value())
            .unwrap_or(0)
    };
    if current == ANALYTICS_VERSION {
        return Ok(());
    }

    log::info!("building analytics indexes and rollups");
    let clear = db.begin_write()?;
    {
        clear.open_table(LOG_TIMES)?.retain(|_, _| false)?;
        clear.open_table(STATUS_LOGS)?.retain(|_, _| false)?;
        clear.open_table(METHOD_LOGS)?.retain(|_, _| false)?;
        clear.open_table(ROLLUPS_MINUTE)?.retain(|_, _| false)?;
        clear.open_table(ROLLUPS_HOUR)?.retain(|_, _| false)?;
        clear.open_table(ROLLUPS_DAY)?.retain(|_, _| false)?;
    }
    clear.commit()?;

    let mut next_key = 0u64;
    let mut migrated = 0usize;
    let mut latest_timestamp = 0.0f64;
    loop {
        let batch: Vec<(u64, LogEntry)> = {
            let rtxn = db.begin_read()?;
            let logs = rtxn.open_table(LOGS)?;
            logs.range(next_key..)?
                .take(MIGRATION_BATCH_SIZE)
                .filter_map(|result| match result {
                    Ok((key, value)) => match serde_json::from_str(value.value()) {
                        Ok(entry) => Some((key.value(), entry)),
                        Err(error) => {
                            log::error!("analytics migration: deserialize row: {error}");
                            None
                        }
                    },
                    Err(error) => {
                        log::error!("analytics migration: read row: {error}");
                        None
                    }
                })
                .collect()
        };
        if batch.is_empty() {
            break;
        }
        next_key = batch
            .last()
            .map(|(id, _)| id.saturating_add(1))
            .unwrap_or(next_key);
        latest_timestamp = batch
            .iter()
            .map(|(_, entry)| entry.ts)
            .fold(latest_timestamp, f64::max);

        let entries: Vec<LogEntry> = batch.iter().map(|(_, entry)| entry.clone()).collect();
        let minute = analytics::build_updates(&entries, analytics::MINUTE_SECS);
        let hour = analytics::build_updates(&entries, analytics::HOUR_SECS);
        let day = analytics::build_updates(&entries, analytics::DAY_SECS);
        let wtxn = db.begin_write()?;
        {
            let mut times = wtxn.open_table(LOG_TIMES)?;
            let mut statuses = wtxn.open_table(STATUS_LOGS)?;
            let mut methods = wtxn.open_table(METHOD_LOGS)?;
            for (id, entry) in &batch {
                times.insert((timestamp_millis(entry.ts), *id), ())?;
                statuses.insert((entry.status, *id), ())?;
                let method = entry.request.method.to_ascii_uppercase();
                methods.insert((method.as_str(), *id), ())?;
            }
            write_rollups(&mut wtxn.open_table(ROLLUPS_MINUTE)?, minute)?;
            write_rollups(&mut wtxn.open_table(ROLLUPS_HOUR)?, hour)?;
            write_rollups(&mut wtxn.open_table(ROLLUPS_DAY)?, day)?;
        }
        wtxn.commit()?;
        migrated += batch.len();
        log::info!("analytics migration: indexed {migrated} entries");
    }

    let wtxn = db.begin_write()?;
    {
        let mut meta = wtxn.open_table(META)?;
        meta.insert(META_ANALYTICS_VERSION, ANALYTICS_VERSION)?;
        if latest_timestamp > 0.0 {
            meta.insert(
                META_COMPACTED_HOUR,
                analytics::bucket_start(latest_timestamp, analytics::HOUR_SECS),
            )?;
            meta.insert(
                META_COMPACTED_DAY,
                analytics::bucket_start(latest_timestamp, analytics::DAY_SECS),
            )?;
        }
    }
    wtxn.commit()?;
    log::info!("analytics indexes ready ({migrated} entries)");
    Ok(())
}

pub fn append_entries(
    db: &Database,
    entries: &[LogEntry],
    file_position: u64,
    inode: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if entries.is_empty() {
        return Ok(());
    }
    let serialized: Vec<String> = entries
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<_, _>>()?;
    let minute = analytics::build_updates(entries, analytics::MINUTE_SECS);
    let affected_hours: BTreeSet<u64> = entries
        .iter()
        .map(|entry| analytics::bucket_start(entry.ts, analytics::HOUR_SECS))
        .collect();

    let wtxn = db.begin_write()?;
    {
        let mut logs = wtxn.open_table(LOGS)?;
        let mut times = wtxn.open_table(LOG_TIMES)?;
        let mut statuses = wtxn.open_table(STATUS_LOGS)?;
        let mut methods = wtxn.open_table(METHOD_LOGS)?;
        let mut meta = wtxn.open_table(META)?;
        let mut next_id = meta
            .get(META_NEXT_ID)?
            .map(|value| value.value())
            .unwrap_or(0);

        for (entry, json) in entries.iter().zip(&serialized) {
            logs.insert(next_id, json.as_str())?;
            times.insert((timestamp_millis(entry.ts), next_id), ())?;
            statuses.insert((entry.status, next_id), ())?;
            let method = entry.request.method.to_ascii_uppercase();
            methods.insert((method.as_str(), next_id), ())?;
            next_id += 1;
        }
        write_rollups(&mut wtxn.open_table(ROLLUPS_MINUTE)?, minute)?;

        meta.insert(META_NEXT_ID, next_id)?;
        meta.insert(META_LAST_POS, file_position)?;
        meta.insert(META_LAST_INODE, inode)?;
    }
    wtxn.commit()?;
    let watermark = entries.iter().map(|entry| entry.ts).fold(0.0, f64::max);
    compact_completed_rollups(db, watermark, affected_hours)?;
    Ok(())
}

fn compact_completed_rollups(
    db: &Database,
    watermark: f64,
    affected_hours: BTreeSet<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    if watermark <= 0.0 {
        return Ok(());
    }
    let current_hour = analytics::bucket_start(watermark, analytics::HOUR_SECS);
    let current_day = analytics::bucket_start(watermark, analytics::DAY_SECS);
    let (hour_marker, day_marker) = {
        let rtxn = db.begin_read()?;
        let meta = rtxn.open_table(META)?;
        (
            meta.get(META_COMPACTED_HOUR)?
                .map(|value| value.value())
                .unwrap_or(0),
            meta.get(META_COMPACTED_DAY)?
                .map(|value| value.value())
                .unwrap_or(0),
        )
    };

    let mut hours_to_rebuild: BTreeSet<u64> = affected_hours
        .into_iter()
        .filter(|bucket| *bucket < current_hour)
        .collect();
    if hour_marker < current_hour {
        let rtxn = db.begin_read()?;
        let minute = rtxn.open_table(ROLLUPS_MINUTE)?;
        for item in minute.range(hour_marker..current_hour)? {
            let (key, _) = item?;
            hours_to_rebuild.insert((key.value() / analytics::HOUR_SECS) * analytics::HOUR_SECS);
        }
    }

    if !hours_to_rebuild.is_empty() || hour_marker < current_hour {
        let mut rebuilt = Vec::new();
        {
            let rtxn = db.begin_read()?;
            let minute = rtxn.open_table(ROLLUPS_MINUTE)?;
            for bucket in &hours_to_rebuild {
                let mut rollup = Rollup::default();
                for item in minute.range(*bucket..(*bucket + analytics::HOUR_SECS))? {
                    let (_, value) = item?;
                    rollup.merge(&serde_json::from_str::<Rollup>(value.value())?);
                }
                rollup.finish_bucket();
                rebuilt.push((*bucket, rollup));
            }
        }
        let wtxn = db.begin_write()?;
        {
            let mut hours = wtxn.open_table(ROLLUPS_HOUR)?;
            for (bucket, rollup) in rebuilt {
                if rollup.total == 0 {
                    hours.remove(bucket)?;
                } else {
                    let json = serde_json::to_string(&rollup)?;
                    hours.insert(bucket, json.as_str())?;
                }
            }
            if hour_marker < current_hour {
                wtxn.open_table(META)?
                    .insert(META_COMPACTED_HOUR, current_hour)?;
            }
        }
        wtxn.commit()?;
    }

    let mut days_to_rebuild: BTreeSet<u64> = hours_to_rebuild
        .into_iter()
        .map(|hour| (hour / analytics::DAY_SECS) * analytics::DAY_SECS)
        .filter(|day| *day < current_day)
        .collect();
    if day_marker < current_day {
        let rtxn = db.begin_read()?;
        let hours = rtxn.open_table(ROLLUPS_HOUR)?;
        for item in hours.range(day_marker..current_day)? {
            let (key, _) = item?;
            days_to_rebuild.insert((key.value() / analytics::DAY_SECS) * analytics::DAY_SECS);
        }
    }

    if !days_to_rebuild.is_empty() || day_marker < current_day {
        let mut rebuilt = Vec::new();
        {
            let rtxn = db.begin_read()?;
            let hours = rtxn.open_table(ROLLUPS_HOUR)?;
            for bucket in &days_to_rebuild {
                let mut rollup = Rollup::default();
                for item in hours.range(*bucket..(*bucket + analytics::DAY_SECS))? {
                    let (_, value) = item?;
                    rollup.merge(&serde_json::from_str::<Rollup>(value.value())?);
                }
                rollup.finish_bucket();
                rebuilt.push((*bucket, rollup));
            }
        }
        let wtxn = db.begin_write()?;
        {
            let mut days = wtxn.open_table(ROLLUPS_DAY)?;
            for (bucket, rollup) in rebuilt {
                if rollup.total == 0 {
                    days.remove(bucket)?;
                } else {
                    let json = serde_json::to_string(&rollup)?;
                    days.insert(bucket, json.as_str())?;
                }
            }
            if day_marker < current_day {
                wtxn.open_table(META)?
                    .insert(META_COMPACTED_DAY, current_day)?;
            }
        }
        wtxn.commit()?;
    }
    Ok(())
}

pub fn analytics_generation(db: &Database) -> u64 {
    (|| -> Result<u64, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let meta = rtxn.open_table(META)?;
        let next_id = meta
            .get(META_NEXT_ID)?
            .map(|value| value.value())
            .unwrap_or(0);
        let revision = meta
            .get(META_DATA_REVISION)?
            .map(|value| value.value())
            .unwrap_or(0);
        Ok(next_id.wrapping_mul(0x9e3779b97f4a7c15) ^ revision.rotate_left(32))
    })()
    .unwrap_or(0)
}

pub fn load_entries_since(db: &Database, since: Option<f64>) -> Result<Vec<LogEntry>, String> {
    (|| -> Result<Vec<_>, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let logs = rtxn.open_table(LOGS)?;
        let mut entries = Vec::new();
        if let Some(start) = since {
            let times = rtxn.open_table(LOG_TIMES)?;
            for result in times.range((timestamp_millis(start), 0)..)? {
                let (key, _) = result?;
                let (_, id) = key.value();
                let Some(value) = logs.get(id)? else { continue };
                match serde_json::from_str::<LogEntry>(value.value()) {
                    Ok(entry) if entry.ts >= start => entries.push(entry),
                    Ok(_) => {}
                    Err(error) => log::error!("load_entries_since: deserialize row: {error}"),
                }
            }
        } else {
            for result in logs.iter()? {
                let (_, value) = result?;
                match serde_json::from_str(value.value()) {
                    Ok(entry) => entries.push(entry),
                    Err(error) => log::error!("load_entries_since: deserialize row: {error}"),
                }
            }
        }
        Ok(entries)
    })()
    .map_err(|e| {
        log::error!("load_entries_since: {e}");
        "Database error".to_string()
    })
}

pub fn load_entries(db: &Database) -> Result<Vec<LogEntry>, String> {
    load_entries_since(db, None)
}

fn merge_rollup_value(
    target: &mut Rollup,
    value: Option<redb::AccessGuard<'_, &str>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(value) = value {
        target.merge(&serde_json::from_str::<Rollup>(value.value())?);
    }
    Ok(())
}

pub fn aggregate_range(db: &Database, since: Option<f64>, until: f64) -> Result<Rollup, String> {
    (|| -> Result<Rollup, Box<dyn std::error::Error>> {
        compact_completed_rollups(db, until, BTreeSet::new())?;
        let rtxn = db.begin_read()?;
        let times = rtxn.open_table(LOG_TIMES)?;
        let logs = rtxn.open_table(LOGS)?;
        let minute = rtxn.open_table(ROLLUPS_MINUTE)?;
        let hour = rtxn.open_table(ROLLUPS_HOUR)?;
        let day = rtxn.open_table(ROLLUPS_DAY)?;
        let Some((first, _)) = times.first()? else {
            return Ok(Rollup::default());
        };
        let start = since.unwrap_or(first.value().0 as f64 / 1_000.0);
        let mut result = Rollup::default();

        for segment in analytics::range_segments(start, until) {
            match segment {
                RangeSegment::Rollup { resolution, bucket } => match resolution {
                    Resolution::Minute => merge_rollup_value(&mut result, minute.get(bucket)?)?,
                    Resolution::Hour => merge_rollup_value(&mut result, hour.get(bucket)?)?,
                    Resolution::Day => merge_rollup_value(&mut result, day.get(bucket)?)?,
                },
                RangeSegment::Raw { start, end } => {
                    let range_start = (timestamp_millis(start), 0);
                    let range_end = (timestamp_millis(end), u64::MAX);
                    for item in times.range(range_start..=range_end)? {
                        let (key, _) = item?;
                        let (_, id) = key.value();
                        let Some(value) = logs.get(id)? else { continue };
                        let entry: LogEntry = serde_json::from_str(value.value())?;
                        if entry.ts >= start && entry.ts < end {
                            result.add_entry(&entry);
                        }
                    }
                }
            }
        }
        result.finish_bucket();
        Ok(result)
    })()
    .map_err(|error| {
        log::error!("aggregate_range: {error}");
        "Database error".to_string()
    })
}

pub fn aggregate_timeline_bucket(
    db: &Database,
    bucket_start: u64,
    bucket_secs: u64,
    cutoff: f64,
    until: f64,
) -> Result<Rollup, String> {
    compact_completed_rollups(db, until, BTreeSet::new()).map_err(|error| {
        log::error!("aggregate_timeline_bucket compaction: {error}");
        "Database error".to_string()
    })?;
    let start = (bucket_start as f64).max(cutoff);
    let end = ((bucket_start + bucket_secs) as f64).min(until);
    if start >= end {
        return Ok(Rollup::default());
    }
    aggregate_range(db, Some(start), end)
}

fn consider_entry<F>(
    id: u64,
    json: &str,
    predicate: &mut F,
    skip: &mut usize,
    limit: usize,
    scanned: &mut usize,
    matches: &mut Vec<(u64, LogEntry)>,
) -> Result<bool, Box<dyn std::error::Error>>
where
    F: FnMut(&LogEntry) -> bool,
{
    *scanned += 1;
    let entry: LogEntry = serde_json::from_str(json)?;
    if !predicate(&entry) {
        return Ok(false);
    }
    if *skip > 0 {
        *skip -= 1;
        return Ok(false);
    }
    matches.push((id, entry));
    Ok(matches.len() > limit)
}

pub fn scan_logs_page<F>(
    db: &Database,
    scan: LogScan,
    cursor: Option<u64>,
    legacy_skip: usize,
    limit: usize,
    include_total: bool,
    mut predicate: F,
) -> Result<LogPage, String>
where
    F: FnMut(&LogEntry) -> bool,
{
    (|| -> Result<LogPage, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let logs = rtxn.open_table(LOGS)?;
        let mut matches = Vec::with_capacity(limit + 1);
        let mut skip = legacy_skip;
        let mut scanned = 0usize;

        match scan {
            LogScan::All => {
                let mut iter = match cursor {
                    Some(cursor) => logs.range(..cursor)?,
                    None => logs.iter()?,
                };
                while let Some(item) = iter.next_back() {
                    let (id, value) = item?;
                    if consider_entry(
                        id.value(),
                        value.value(),
                        &mut predicate,
                        &mut skip,
                        limit,
                        &mut scanned,
                        &mut matches,
                    )? {
                        break;
                    }
                }
            }
            LogScan::Status(status) => {
                let statuses = rtxn.open_table(STATUS_LOGS)?;
                let mut iter = match cursor {
                    Some(cursor) => statuses.range((status, 0)..(status, cursor))?,
                    None => statuses.range((status, 0)..=(status, u64::MAX))?,
                };
                while let Some(item) = iter.next_back() {
                    let (key, _) = item?;
                    let (_, id) = key.value();
                    let Some(value) = logs.get(id)? else { continue };
                    if consider_entry(
                        id,
                        value.value(),
                        &mut predicate,
                        &mut skip,
                        limit,
                        &mut scanned,
                        &mut matches,
                    )? {
                        break;
                    }
                }
            }
            LogScan::Method(method) => {
                let methods = rtxn.open_table(METHOD_LOGS)?;
                let mut iter = match cursor {
                    Some(cursor) => {
                        methods.range((method.as_str(), 0)..(method.as_str(), cursor))?
                    }
                    None => methods.range((method.as_str(), 0)..=(method.as_str(), u64::MAX))?,
                };
                while let Some(item) = iter.next_back() {
                    let (key, _) = item?;
                    let (_, id) = key.value();
                    let Some(value) = logs.get(id)? else { continue };
                    if consider_entry(
                        id,
                        value.value(),
                        &mut predicate,
                        &mut skip,
                        limit,
                        &mut scanned,
                        &mut matches,
                    )? {
                        break;
                    }
                }
            }
        }

        let has_more = matches.len() > limit;
        if has_more {
            matches.pop();
        }
        let next_cursor = has_more
            .then(|| matches.last().map(|(id, _)| *id))
            .flatten();
        let entries = matches.into_iter().map(|(_, entry)| entry).collect();
        let total = if include_total {
            Some(logs.len()?)
        } else {
            None
        };
        Ok(LogPage {
            entries,
            next_cursor,
            has_more,
            total,
            scanned,
        })
    })()
    .map_err(|error| {
        log::error!("scan_logs_page: {error}");
        "Database error".to_string()
    })
}

pub fn visit_logs_newest<F>(db: &Database, mut visitor: F) -> Result<(), String>
where
    F: FnMut(&LogEntry),
{
    (|| -> Result<(), Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let logs = rtxn.open_table(LOGS)?;
        let mut iter = logs.iter()?;
        while let Some(item) = iter.next_back() {
            let (_, value) = item?;
            match serde_json::from_str::<LogEntry>(value.value()) {
                Ok(entry) => visitor(&entry),
                Err(error) => log::error!("visit_logs_newest: deserialize row: {error}"),
            }
        }
        Ok(())
    })()
    .map_err(|error| {
        log::error!("visit_logs_newest: {error}");
        "Database error".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_parser::RequestInfo;

    fn entry(ts: f64, status: u16, method: &str) -> LogEntry {
        LogEntry {
            ts,
            request: RequestInfo {
                remote_ip: "192.0.2.1".to_string(),
                remote_port: "443".to_string(),
                client_ip: "198.51.100.2".to_string(),
                proto: "HTTP/2.0".to_string(),
                method: method.to_string(),
                host: "example.test".to_string(),
                uri: format!("/{status}"),
                headers: HashMap::new(),
                tls: None,
            },
            duration: 0.025,
            size: 512,
            status,
            bytes_read: 0,
            user_id: String::new(),
            resp_headers: HashMap::new(),
        }
    }

    #[test]
    fn indexed_pages_rollups_and_retention_stay_consistent() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "caddy-dashboard-db-test-{}-{unique}",
            std::process::id()
        ));
        let db = open(path.to_str().unwrap());
        let entries = vec![
            entry(1_000.0, 200, "GET"),
            entry(1_060.0, 404, "GET"),
            entry(1_120.0, 500, "POST"),
        ];
        append_entries(&db, &entries, 10, 20).unwrap();

        let aggregate = aggregate_range(&db, Some(1_000.0), 1_200.0).unwrap();
        assert_eq!(aggregate.total, 3);
        assert_eq!(aggregate.status_codes.get(&404), Some(&1));

        let first = scan_logs_page(&db, LogScan::All, None, 0, 2, true, |_| true).unwrap();
        assert_eq!(first.entries.len(), 2);
        assert_eq!(first.entries[0].status, 500);
        assert!(first.has_more);
        assert_eq!(first.total, Some(3));

        let second =
            scan_logs_page(&db, LogScan::All, first.next_cursor, 0, 2, true, |_| true).unwrap();
        assert_eq!(second.entries.len(), 1);
        assert_eq!(second.entries[0].status, 200);

        let status = scan_logs_page(&db, LogScan::Status(404), None, 0, 10, false, |entry| {
            entry.status == 404
        })
        .unwrap();
        assert_eq!(status.entries.len(), 1);
        assert!(status.scanned <= 1);

        append_entries(&db, &[entry(4_000.0, 201, "PUT")], 11, 20).unwrap();
        let completed_hour = aggregate_range(&db, Some(0.0), 3_600.0).unwrap();
        assert_eq!(completed_hour.total, 3);

        append_entries(&db, &[entry(90_000.0, 202, "PATCH")], 12, 20).unwrap();
        let completed_day = aggregate_range(&db, Some(0.0), 86_400.0).unwrap();
        assert_eq!(completed_day.total, 4);

        assert_eq!(purge_old(&db, 1_060.5), 2);
        let retained = aggregate_range(&db, None, 100_000.0).unwrap();
        assert_eq!(retained.total, 3);
        assert_eq!(retained.status_codes.get(&500), Some(&1));

        drop(db);
        std::fs::remove_dir_all(path).unwrap();
    }
}
