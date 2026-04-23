use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::Arc;

pub const LOGS: TableDefinition<u64, &str> = TableDefinition::new("logs");
pub const META: TableDefinition<u64, u64> = TableDefinition::new("meta");

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
    wtxn.commit().expect("failed to commit table init");
    Arc::new(db)
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
    .unwrap_or(0)
}

pub fn load_entries(db: &Database) -> Vec<crate::log_parser::LogEntry> {
    (|| -> Result<Vec<_>, Box<dyn std::error::Error>> {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(LOGS)?;
        let mut entries = Vec::new();
        for result in table.iter()? {
            let (_, v) = result?;
            if let Ok(entry) = serde_json::from_str(v.value()) {
                entries.push(entry);
            }
        }
        Ok(entries)
    })()
    .unwrap_or_default()
}
