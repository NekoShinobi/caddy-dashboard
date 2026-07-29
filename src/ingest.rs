use crate::db::{META, META_LAST_INODE, META_LAST_POS};
use crate::log_parser::LogEntry;
use redb::{Database, ReadableDatabase};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

pub async fn run(db: Arc<Database>, tx: broadcast::Sender<LogEntry>) {
    loop {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Err(e) = ingest_batch(&db, &tx) {
            log::warn!("ingest error: {e}");
        }
    }
}

fn ingest_batch(
    db: &Database,
    tx: &broadcast::Sender<LogEntry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = crate::env::LOG_PATH.as_str();

    let Ok(file_meta) = std::fs::metadata(path) else {
        return Ok(());
    };

    let (mut last_pos, last_inode) = {
        let rtxn = db.begin_read()?;
        let table = rtxn.open_table(META)?;
        let pos = table.get(META_LAST_POS)?.map(|v| v.value()).unwrap_or(0);
        let inode = table.get(META_LAST_INODE)?.map(|v| v.value()).unwrap_or(0);
        (pos, inode)
    };

    // Rotation detection: inode changed or file truncated
    if last_inode != 0 && (file_meta.ino() != last_inode || file_meta.len() < last_pos) {
        last_pos = 0;
    }

    // First-ever start: skip existing content, tail only new entries going forward
    if last_inode == 0 {
        let wtxn = db.begin_write()?;
        {
            let mut meta = wtxn.open_table(META)?;
            meta.insert(META_LAST_POS, file_meta.len())?;
            meta.insert(META_LAST_INODE, file_meta.ino())?;
        }
        wtxn.commit()?;
        return Ok(());
    }

    if file_meta.len() <= last_pos {
        return Ok(());
    }

    let Ok(mut file) = std::fs::File::open(path) else {
        return Ok(());
    };
    if file.seek(SeekFrom::Start(last_pos)).is_err() {
        return Ok(());
    }

    let mut reader = BufReader::new(file);
    let mut entries: Vec<LogEntry> = Vec::new();
    let mut pos = last_pos;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(n) => {
                pos += n as u64;
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    match serde_json::from_str(trimmed) {
                        Ok(entry) => entries.push(entry),
                        Err(e) => {
                            log::error!("ingest: failed to parse log line: {e} — line: {trimmed}")
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("ingest: read_line error: {e}");
                break;
            }
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    crate::db::append_entries(db, &entries, pos, file_meta.ino())?;

    for entry in entries {
        let _ = tx.send(entry);
    }

    Ok(())
}
