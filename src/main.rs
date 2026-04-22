use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use tokio::sync::broadcast;

mod db;
mod env;
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

    log::info!("starting caddy-dashboard on port {}", *env::PORT);
    web::start(db, tx).await
}
