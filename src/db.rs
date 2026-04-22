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
