use actix_web::web;

mod ai;
mod geo;
mod logs;
mod reports;
mod stats;
mod timeline;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(stats::get_stats)
            .service(logs::get_logs)
            .service(logs::export_logs_csv)
            .service(logs::stream_logs)
            .service(timeline::get_timeline)
            .service(geo::get_geo)
            .service(reports::get_error_rates)
            .service(reports::get_large_payloads)
            .service(ai::get_ai_analysis),
    );
}
