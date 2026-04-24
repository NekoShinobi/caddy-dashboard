use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use tokio::sync::broadcast;

mod db;
mod env;
mod geoip;
mod ingest;
mod log_parser;
mod web;

#[tokio::main]
async fn main() -> miette::Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .expect("logger init failed");

    log::info!("opening database at {}", *env::DATA_DIR);
    let db = db::open(&env::DATA_DIR);

    let (tx, _) = broadcast::channel::<log_parser::LogEntry>(1024);

    let db_ingest = db.clone();
    let tx_ingest = tx.clone();
    tokio::spawn(async move { ingest::run(db_ingest, tx_ingest).await });

    if *env::RETENTION_DAYS > 0 {
        let db_retention = db.clone();
        let days = *env::RETENTION_DAYS;
        log::info!("retention policy enabled: purging entries older than {days} days");
        tokio::spawn(async move {
            loop {
                let cutoff = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
                    - (days as f64 * 86_400.0);
                let purged = db::purge_old(&db_retention, cutoff);
                if purged > 0 {
                    log::info!("retention: purged {purged} entries older than {days} days");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(3_600)).await;
            }
        });
    }

    let geoip = geoip::open();

    log::info!(
        "Ollama: host={} model={}",
        *env::OLLAMA_HOST,
        *env::OLLAMA_MODEL
    );

    log::info!("starting caddy-dashboard on port {}", *env::PORT);
    web::start(db, tx, geoip).await
}
