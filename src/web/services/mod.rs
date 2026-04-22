use actix_web::web;

mod geo;
mod logs;
mod stats;
mod timeline;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(stats::get_stats)
            .service(logs::get_logs)
            .service(logs::stream_logs)
            .service(timeline::get_timeline)
            .service(geo::get_geo),
    );
}
