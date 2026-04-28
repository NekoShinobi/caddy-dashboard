use actix_web::web;

mod admin;
pub mod ai;
mod auth;
mod geo;
mod logs;
mod oidc;
mod reports;
mod settings;
mod stats;
mod timeline;

async fn api_not_found() -> actix_web::HttpResponse {
    actix_web::HttpResponse::NotFound()
        .json(serde_json::json!({"error": "not found"}))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .wrap(crate::web::middleware::RequireAuth)
            .default_service(web::to(api_not_found))
            // Auth endpoints — exempt from RequireAuth via middleware path check
            .service(auth::me)
            .service(auth::login)
            .service(auth::logout)
            .service(auth::signup)
            .service(auth::change_password)
            // OIDC endpoints — also exempt (/api/auth/ prefix)
            .service(oidc::config)
            .service(oidc::login)
            .service(oidc::callback)
            // Admin endpoints
            .service(admin::list_users)
            .service(admin::create_user)
            .service(admin::edit_user)
            .service(admin::delete_user)
            .service(admin::reset_password)
            // Site settings endpoints
            .service(settings::get_ai_prompt)
            .service(settings::put_ai_prompt)
            // App endpoints
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
