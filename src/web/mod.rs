use actix_files::{Files, NamedFile};
use actix_web::{web, App, HttpRequest, HttpServer, Result};
use redb::Database;
use std::sync::Arc;
use tokio::sync::broadcast;

mod middleware;
mod services;

async fn spa_fallback(_req: HttpRequest) -> Result<NamedFile> {
    Ok(NamedFile::open("./static/index.html")?)
}

pub async fn start(
    db: Arc<Database>,
    tx: broadcast::Sender<crate::log_parser::LogEntry>,
    geoip: crate::geoip::GeoIpDb,
) -> miette::Result<()> {
    let port = *crate::env::PORT;
    std::fs::create_dir_all("./static")
        .map_err(|e| miette::miette!("failed to create ./static: {e}"))?;

    let db_data = web::Data::from(db);
    let tx_data = web::Data::new(tx);
    let geoip_data = web::Data::new(geoip);
    let throttle = web::Data::new(crate::login_throttle::LoginThrottle::new());

    log::info!(
        "session cookies: secure={}",
        *crate::env::COOKIE_SECURE
    );

    HttpServer::new(move || {
        App::new()
            .app_data(db_data.clone())
            .app_data(tx_data.clone())
            .app_data(geoip_data.clone())
            .app_data(throttle.clone())
            .configure(services::configure)
            .service(Files::new("/", "./static").index_file("index.html"))
            .default_service(web::get().to(spa_fallback))
    })
    .bind(("0.0.0.0", port))
    .map_err(|e| miette::miette!("bind failed: {e}"))?
    .run()
    .await
    .map_err(|e| miette::miette!("server error: {e}"))
}
