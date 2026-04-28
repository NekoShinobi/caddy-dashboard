use actix_web::{get, put, web, HttpRequest, HttpResponse};
use redb::Database;
use serde::{Deserialize, Serialize};

fn require_admin(req: &HttpRequest, db: &Database) -> Result<(), HttpResponse> {
    let username = crate::session::get_username(req, db)
        .ok_or_else(|| HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})))?;
    let user = crate::db::get_user(db, &username)
        .ok_or_else(|| {
            log::error!("settings require_admin: session valid but user '{username}' not found");
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}))
        })?;
    if !user.is_admin {
        log::warn!("settings require_admin: non-admin '{username}' attempted settings access");
        return Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "Admin required"})));
    }
    Ok(())
}

#[derive(Serialize)]
struct PromptResponse {
    template: String,
    default: &'static str,
}

#[get("/admin/settings/ai-prompt")]
pub async fn get_ai_prompt(req: HttpRequest, db: web::Data<Database>) -> HttpResponse {
    if let Err(e) = require_admin(&req, &db) { return e; }
    let template = crate::db::get_setting(&db, crate::web::services::ai::SETTING_KEY)
        .unwrap_or_else(|| crate::web::services::ai::DEFAULT_PROMPT_TEMPLATE.to_string());
    HttpResponse::Ok().json(PromptResponse {
        template,
        default: crate::web::services::ai::DEFAULT_PROMPT_TEMPLATE,
    })
}

#[derive(Deserialize)]
pub struct UpdatePromptBody {
    template: String,
}

#[put("/admin/settings/ai-prompt")]
pub async fn put_ai_prompt(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<UpdatePromptBody>,
) -> HttpResponse {
    if let Err(e) = require_admin(&req, &db) { return e; }
    if !body.template.contains("{summary}") {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Template must contain the {summary} placeholder"}));
    }
    if !crate::db::set_setting(&db, crate::web::services::ai::SETTING_KEY, &body.template) {
        log::error!("put_ai_prompt: set_setting failed");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to save setting"}));
    }
    log::info!("put_ai_prompt: AI prompt template updated");
    HttpResponse::Ok().json(serde_json::json!({"ok": true}))
}
