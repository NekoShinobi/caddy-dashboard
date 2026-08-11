use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use redb::Database;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct UserRow {
    username: String,
    email: String,
    is_admin: bool,
    created_at: f64,
    is_oidc: bool,
}

fn require_admin(req: &HttpRequest, db: &Database) -> Result<crate::auth::User, HttpResponse> {
    let username = crate::session::get_username(req, db).ok_or_else(|| {
        HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}))
    })?;
    let user = crate::db::get_user(db, &username).ok_or_else(|| {
        log::error!("require_admin: session valid but user '{username}' not found in DB");
        HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}))
    })?;
    if !user.is_admin {
        log::warn!("require_admin: user '{username}' attempted admin action without admin rights");
        return Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "Admin required"})));
    }
    Ok(user)
}

#[get("/admin/users")]
pub async fn list_users(req: HttpRequest, db: web::Data<Database>) -> HttpResponse {
    if let Err(e) = require_admin(&req, &db) {
        return e;
    }
    let users: Vec<UserRow> = crate::db::list_users(&db)
        .into_iter()
        .map(|u| {
            let is_oidc = u.password_hash.starts_with("oidc:");
            UserRow {
                username: u.username,
                email: u.email,
                is_admin: u.is_admin,
                created_at: u.created_at,
                is_oidc,
            }
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({"users": users}))
}

#[derive(Deserialize)]
pub struct CreateUserBody {
    username: String,
    email: String,
    password: String,
    is_admin: bool,
}

#[post("/admin/users")]
pub async fn create_user(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateUserBody>,
) -> HttpResponse {
    let admin = match require_admin(&req, &db) {
        Ok(u) => u,
        Err(e) => return e,
    };
    if body.username.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Username required"}));
    }
    if crate::db::get_user(&db, body.username.trim()).is_some() {
        return HttpResponse::Conflict()
            .json(serde_json::json!({"error": "Username already exists"}));
    }
    if body.password.len() < 8 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Password must be at least 8 characters"}));
    }
    let hash = match crate::auth::hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => {
            log::error!(
                "admin create_user({}): hash_password failed: {e}",
                body.username
            );
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal error"}));
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let user = crate::auth::User {
        username: body.username.trim().to_string(),
        email: body.email.trim().to_string(),
        password_hash: hash,
        is_admin: body.is_admin,
        created_at: now,
    };
    if !crate::db::create_user(&db, &user) {
        log::error!("admin create_user({}): DB write failed", user.username);
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to create user"}));
    }
    log::info!(
        "admin '{}': created user '{}'",
        admin.username,
        user.username
    );
    HttpResponse::Created().json(serde_json::json!({"ok": true, "username": user.username}))
}

#[delete("/admin/users/{username}")]
pub async fn delete_user(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> HttpResponse {
    let admin = match require_admin(&req, &db) {
        Ok(u) => u,
        Err(e) => return e,
    };
    let target = path.into_inner();
    if target == admin.username {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Cannot delete your own account"}));
    }
    if crate::db::get_user(&db, &target).is_none() {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "User not found"}));
    }
    let users = crate::db::list_users(&db);
    let target_user = users.iter().find(|u| u.username == target);
    if target_user.map(|u| u.is_admin).unwrap_or(false) {
        let admin_count = users.iter().filter(|u| u.is_admin).count();
        if admin_count <= 1 {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "Cannot delete the last admin"}));
        }
    }
    crate::db::delete_user_sessions(&db, &target);
    crate::db::delete_user(&db, &target);
    log::info!("admin '{}': deleted user '{target}'", admin.username);
    HttpResponse::NoContent().finish()
}

#[derive(Deserialize)]
pub struct EditUserBody {
    username: Option<String>,
    email: Option<String>,
    is_admin: Option<bool>,
}

#[put("/admin/users/{username}")]
pub async fn edit_user(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<EditUserBody>,
) -> HttpResponse {
    let admin = match require_admin(&req, &db) {
        Ok(u) => u,
        Err(e) => return e,
    };
    let old_username = path.into_inner();
    let Some(mut user) = crate::db::get_user(&db, &old_username) else {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "User not found"}));
    };

    if let Some(new_admin) = body.is_admin {
        if user.is_admin && !new_admin {
            let admin_count = crate::db::list_users(&db)
                .iter()
                .filter(|u| u.is_admin)
                .count();
            if admin_count <= 1 {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Cannot remove the last admin"}));
            }
        }
        user.is_admin = new_admin;
    }

    if let Some(email) = &body.email {
        user.email = email.trim().to_string();
    }

    let username_changed = if let Some(new_username) = &body.username {
        let new_username = new_username.trim().to_string();
        if new_username.is_empty() {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "Username cannot be empty"}));
        }
        if new_username != old_username && crate::db::get_user(&db, &new_username).is_some() {
            return HttpResponse::Conflict()
                .json(serde_json::json!({"error": "Username already exists"}));
        }
        let changed = new_username != old_username;
        user.username = new_username;
        changed
    } else {
        false
    };

    if !crate::db::update_user(&db, &old_username, &user) {
        log::error!("admin edit_user({old_username}): DB write failed");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to update user"}));
    }
    log::info!(
        "admin '{}': edited user '{old_username}' -> '{}'",
        admin.username,
        user.username
    );

    if username_changed {
        crate::db::delete_user_sessions(&db, &old_username);
        if old_username == admin.username {
            let token = crate::session::generate_token();
            if !crate::db::create_session(&db, &token, &user.username) {
                log::error!(
                    "edit_user: failed to create new session for renamed admin '{}'",
                    user.username
                );
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to create session"}));
            }
            return HttpResponse::Ok()
                .cookie(crate::session::make_cookie(&token))
                .json(serde_json::json!({"ok": true, "username": user.username}));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({"ok": true, "username": user.username}))
}

#[derive(Deserialize)]
pub struct ResetPasswordBody {
    new_password: String,
}

#[put("/admin/users/{username}/password")]
pub async fn reset_password(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<ResetPasswordBody>,
) -> HttpResponse {
    let admin = match require_admin(&req, &db) {
        Ok(u) => u,
        Err(e) => return e,
    };
    if body.new_password.len() < 8 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Password must be at least 8 characters"}));
    }
    let username = path.into_inner();
    if crate::db::get_user(&db, &username).is_none() {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "User not found"}));
    }
    let hash = match crate::auth::hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => {
            log::error!("admin reset_password({username}): hash_password failed: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal error"}));
        }
    };
    if !crate::db::update_password(&db, &username, &hash) {
        log::error!("admin reset_password({username}): DB write failed");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to update password"}));
    }
    crate::db::delete_user_sessions(&db, &username);
    log::info!(
        "admin '{}': reset password for '{username}'",
        admin.username
    );
    HttpResponse::Ok().json(serde_json::json!({"ok": true}))
}
