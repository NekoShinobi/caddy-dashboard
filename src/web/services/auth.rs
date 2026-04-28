use actix_web::{get, post, put, web, HttpRequest, HttpResponse};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

async fn oidc_logout_url(id_token: Option<&str>, req: &HttpRequest) -> Option<String> {
    let disc = crate::oidc::fetch_discovery().await.ok()?;
    let endpoint = disc.end_session_endpoint.as_deref()?;
    let base = crate::env::BASE_URL.as_deref().map(|s| s.to_string()).unwrap_or_else(|| {
        let ci = req.connection_info();
        format!("{}://{}", ci.scheme(), ci.host())
    });
    let post_logout = urlencoding::encode(&base);
    Some(if let Some(tok) = id_token {
        format!("{endpoint}?id_token_hint={}&post_logout_redirect_uri={post_logout}", urlencoding::encode(tok))
    } else {
        format!("{endpoint}?post_logout_redirect_uri={post_logout}")
    })
}

#[derive(Serialize)]
struct UserResponse {
    username: String,
    email: String,
    is_admin: bool,
    created_at: f64,
    is_oidc: bool,
}

impl From<crate::auth::User> for UserResponse {
    fn from(u: crate::auth::User) -> Self {
        let is_oidc = u.password_hash.starts_with("oidc:");
        Self { username: u.username, email: u.email, is_admin: u.is_admin, created_at: u.created_at, is_oidc }
    }
}

// Pre-computed dummy hash used to ensure constant-time response when a
// username does not exist, preventing timing-based user enumeration.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    crate::auth::hash_password("__sentinel_never_valid_xK9$mQ2#__")
        .inspect_err(|e| log::error!("DUMMY_HASH: failed to hash sentinel password: {e}"))
        .unwrap_or_default()
});

#[get("/auth/me")]
pub async fn me(req: HttpRequest, db: web::Data<Database>) -> HttpResponse {
    if let Some(username) = crate::session::get_username(&req, &db) {
        if let Some(user) = crate::db::get_user(&db, &username) {
            return HttpResponse::Ok().json(UserResponse::from(user));
        }
        // Session token exists but user was deleted; clean up the stale session
        if let Some(token) = crate::session::get_token(&req) {
            crate::db::delete_session(&db, &token);
        }
    }
    let needs_setup = crate::db::user_count(&db) == 0;
    HttpResponse::Unauthorized()
        .cookie(crate::session::clear_cookie())
        .json(serde_json::json!({ "needs_setup": needs_setup }))
}

#[derive(Deserialize)]
pub struct LoginBody {
    username: String,
    password: String,
}

#[post("/auth/login")]
pub async fn login(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<LoginBody>,
    throttle: web::Data<crate::login_throttle::LoginThrottle>,
) -> HttpResponse {
    if *crate::env::OIDC_DISABLE_LOGIN {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Local login is disabled. Use SSO to sign in."}));
    }
    // Apply progressive delay for accounts with prior failures
    let prior_failures = throttle.fail_count(&body.username);
    if prior_failures > 10 {
        log::warn!("login: throttling '{}' for 5s ({prior_failures} failures)", body.username);
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    } else if prior_failures > 5 {
        log::warn!("login: throttling '{}' for 1s ({prior_failures} failures)", body.username);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Always run verify_password even when the user doesn't exist so that
    // response time is identical regardless of username validity.
    let (user_opt, hash) = match crate::db::get_user(&db, &body.username) {
        Some(u) => { let h = u.password_hash.clone(); (Some(u), h) }
        None    => (None, DUMMY_HASH.clone()),
    };

    let valid = crate::auth::verify_password(&body.password, &hash);

    if user_opt.is_none() || !valid {
        let count = throttle.record_failure(&body.username);
        log::warn!("login: failed attempt for '{}' (total failures: {count})", body.username);
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "Invalid credentials"}));
    }

    let user = user_opt.unwrap();
    throttle.record_success(&user.username);
    log::info!("login: '{}' authenticated successfully", user.username);

    // Invalidate any existing session for this browser before issuing a new one
    if let Some(old_token) = crate::session::get_token(&req) {
        crate::db::delete_session(&db, &old_token);
    }

    let token = crate::session::generate_token();
    if !crate::db::create_session(&db, &token, &user.username) {
        log::error!("login: failed to create session for '{}'", user.username);
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to create session"}));
    }

    HttpResponse::Ok()
        .cookie(crate::session::make_cookie(&token))
        .json(UserResponse::from(user))
}

#[post("/auth/logout")]
pub async fn logout(req: HttpRequest, db: web::Data<Database>) -> HttpResponse {
    let mut oidc_id_token: Option<String> = None;
    if let Some(token) = crate::session::get_token(&req) {
        if crate::oidc::is_enabled() {
            oidc_id_token = crate::db::get_oidc_token(&db, &token);
        }
        crate::db::delete_session(&db, &token);
    }
    let logout_url = if crate::oidc::is_enabled() {
        oidc_logout_url(oidc_id_token.as_deref(), &req).await
    } else {
        None
    };
    HttpResponse::Ok()
        .cookie(crate::session::clear_cookie())
        .json(serde_json::json!({"ok": true, "logout_url": logout_url}))
}

#[derive(Deserialize)]
pub struct SignupBody {
    username: String,
    email: String,
    password: String,
}

#[post("/auth/signup")]
pub async fn signup(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<SignupBody>,
) -> HttpResponse {
    if *crate::env::OIDC_DISABLE_LOGIN && crate::oidc::is_enabled() {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Local accounts are disabled. Use SSO to sign in."}));
    }
    if crate::db::user_count(&db) > 0 {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Setup already complete"}));
    }
    if body.username.trim().is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Username required"}));
    }
    if body.password.len() < 8 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Password must be at least 8 characters"}));
    }
    let hash = match crate::auth::hash_password(&body.password) {
        Ok(h) => h,
        Err(e) => {
            log::error!("signup: hash_password failed: {e}");
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
        is_admin: true,
        created_at: now,
    };
    if !crate::db::create_user(&db, &user) {
        log::error!("signup: failed to create user '{}'", user.username);
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to create user"}));
    }
    log::info!("signup: first user '{}' created", user.username);

    // Clear any stale session cookie before issuing the new one
    if let Some(old_token) = crate::session::get_token(&req) {
        crate::db::delete_session(&db, &old_token);
    }

    let token = crate::session::generate_token();
    if !crate::db::create_session(&db, &token, &user.username) {
        log::error!("signup: failed to create session for '{}'", user.username);
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to create session"}));
    }

    HttpResponse::Created()
        .cookie(crate::session::make_cookie(&token))
        .json(UserResponse::from(user))
}

#[derive(Deserialize)]
pub struct ChangePasswordBody {
    current_password: String,
    new_password: String,
}

#[put("/auth/password")]
pub async fn change_password(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<ChangePasswordBody>,
) -> HttpResponse {
    let Some(username) = crate::session::get_username(&req, &db) else {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "unauthorized"}));
    };
    let Some(user) = crate::db::get_user(&db, &username) else {
        log::error!("change_password: session valid but user '{username}' not found in DB");
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "unauthorized"}));
    };
    if user.password_hash.starts_with("oidc:") {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "OIDC-managed accounts cannot change their password here"}));
    }
    if !crate::auth::verify_password(&body.current_password, &user.password_hash) {
        log::warn!("change_password: wrong current password for '{username}'");
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "Current password is incorrect"}));
    }
    if body.new_password.len() < 8 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Password must be at least 8 characters"}));
    }
    let hash = match crate::auth::hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => {
            log::error!("change_password({username}): hash_password failed: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal error"}));
        }
    };
    if !crate::db::update_password(&db, &username, &hash) {
        log::error!("change_password({username}): update_password failed");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to update password"}));
    }

    // Invalidate all sessions for this user, then issue a fresh one for the
    // current device so the user stays logged in after changing their password.
    crate::db::delete_user_sessions(&db, &username);
    let token = crate::session::generate_token();
    if !crate::db::create_session(&db, &token, &username) {
        log::error!("change_password({username}): failed to create new session");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Failed to create session"}));
    }
    log::info!("change_password: password updated for '{username}'");

    HttpResponse::Ok()
        .cookie(crate::session::make_cookie(&token))
        .json(serde_json::json!({"ok": true}))
}
