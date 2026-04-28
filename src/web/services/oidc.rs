use actix_web::{get, web, HttpRequest, HttpResponse};
use redb::Database;
use serde::Deserialize;

const STATE_COOKIE: &str = "cd_oidc_state";

fn redirect_uri(req: &HttpRequest) -> String {
    if let Some(base) = crate::env::BASE_URL.as_deref() {
        return format!("{base}/api/auth/oidc/callback");
    }
    let ci = req.connection_info();
    format!("{}://{}/api/auth/oidc/callback", ci.scheme(), ci.host())
}

fn redirect_error(code: &str) -> HttpResponse {
    HttpResponse::Found()
        .append_header(("Location", format!("/?oidc_error={code}")))
        .finish()
}

#[get("/auth/oidc/config")]
pub async fn config() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "enabled": crate::oidc::is_enabled(),
        "provider_name": *crate::env::OIDC_PROVIDER_NAME,
        "logo_url": *crate::env::OIDC_PROVIDER_LOGO_URL,
        "disable_login": *crate::env::OIDC_DISABLE_LOGIN,
    }))
}

#[get("/auth/oidc/login")]
pub async fn login(req: HttpRequest) -> HttpResponse {
    if !crate::oidc::is_enabled() {
        return HttpResponse::NotFound()
            .json(serde_json::json!({"error": "OIDC not configured"}));
    }
    let disc = match crate::oidc::fetch_discovery().await {
        Ok(d) => d,
        Err(e) => {
            log::error!("OIDC login: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "OIDC configuration error"}));
        }
    };

    let state = crate::session::generate_token();
    let redir = redirect_uri(&req);
    let client_id = crate::env::OIDC_CLIENT_ID.as_deref().unwrap_or("");
    let scope = crate::env::OIDC_SCOPES.as_str();

    let url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        disc.authorization_endpoint,
        urlencoding::encode(client_id),
        urlencoding::encode(&redir),
        urlencoding::encode(scope),
        state,
    );

    let state_cookie = actix_web::cookie::Cookie::build(STATE_COOKIE, state)
        .path("/")
        .http_only(true)
        .same_site(actix_web::cookie::SameSite::Lax)
        .secure(*crate::env::COOKIE_SECURE)
        .max_age(actix_web::cookie::time::Duration::minutes(10))
        .finish();

    HttpResponse::Found()
        .cookie(state_cookie)
        .append_header(("Location", url))
        .finish()
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[get("/auth/oidc/callback")]
pub async fn callback(
    req: HttpRequest,
    db: web::Data<Database>,
    query: web::Query<CallbackQuery>,
) -> HttpResponse {
    if !crate::oidc::is_enabled() {
        return HttpResponse::NotFound()
            .json(serde_json::json!({"error": "OIDC not configured"}));
    }

    if let Some(err) = &query.error {
        let desc = query.error_description.as_deref().unwrap_or("");
        log::warn!("OIDC callback: provider error: {err} — {desc}");
        return redirect_error("provider_error");
    }

    let got_state = query.state.as_deref().unwrap_or("");
    let expected_state = req
        .cookie(STATE_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    if got_state.is_empty() || expected_state.is_empty() || got_state != expected_state {
        log::warn!("OIDC callback: state mismatch");
        return redirect_error("state_mismatch");
    }

    let code = match query.code.as_deref() {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => return redirect_error("missing_code"),
    };

    let disc = match crate::oidc::fetch_discovery().await {
        Ok(d) => d,
        Err(e) => {
            log::error!("OIDC callback: discovery: {e}");
            return redirect_error("config_error");
        }
    };

    let redir = redirect_uri(&req);
    let tokens = match crate::oidc::exchange_code(&disc, &code, &redir).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("OIDC callback: token exchange: {e}");
            return redirect_error("token_exchange_failed");
        }
    };

    let userinfo = match crate::oidc::get_userinfo(&disc, &tokens.access_token).await {
        Ok(u) => u,
        Err(e) => {
            log::error!("OIDC callback: userinfo: {e}");
            return redirect_error("userinfo_failed");
        }
    };

    // Reject explicitly unverified emails; absent claim is treated as verified
    if let Some(false) = userinfo.email_verified {
        log::warn!("OIDC callback: unverified email for sub={}", userinfo.sub);
        return redirect_error("email_not_verified");
    }

    let email = match userinfo.email.as_deref() {
        Some(e) if !e.is_empty() => e.to_string(),
        _ => {
            log::warn!("OIDC callback: no email for sub={}", userinfo.sub);
            return redirect_error("no_email");
        }
    };

    let existing = crate::db::get_user_by_email(&db, &email);
    let is_first_user = crate::db::user_count(&db) == 0;

    let is_admin = if crate::env::OIDC_ADMIN_CLAIM.is_some() {
        userinfo.check_admin()
    } else if let Some(ref u) = existing {
        u.is_admin
    } else {
        is_first_user
    };

    let user = if let Some(mut u) = existing {
        if crate::env::OIDC_ADMIN_CLAIM.is_some() && u.is_admin != is_admin {
            u.is_admin = is_admin;
            let old = u.username.clone();
            crate::db::update_user(&db, &old, &u);
        }
        u
    } else {
        let username = derive_username(&db, &userinfo);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let new_user = crate::auth::User {
            username: username.clone(),
            email: email.clone(),
            password_hash: format!("oidc:{}", userinfo.sub),
            is_admin,
            created_at: now,
        };
        if !crate::db::create_user(&db, &new_user) {
            log::error!("OIDC: failed to create user for {email}");
            return redirect_error("create_user_failed");
        }
        log::info!("OIDC: new user '{}' ({})", username, email);
        new_user
    };

    if let Some(old_tok) = crate::session::get_token(&req) {
        crate::db::delete_session(&db, &old_tok);
    }
    let token = crate::session::generate_token();
    if !crate::db::create_session(&db, &token, &user.username) {
        log::error!("OIDC: failed to create session for '{}'", user.username);
        return redirect_error("session_failed");
    }
    // Store the id_token so the logout handler can pass it as id_token_hint
    if let Some(id_tok) = &tokens.id_token {
        crate::db::store_oidc_token(&db, &token, id_tok);
    }

    log::info!("OIDC: '{}' authenticated", user.username);

    let clear_state = actix_web::cookie::Cookie::build(STATE_COOKIE, "")
        .path("/")
        .max_age(actix_web::cookie::time::Duration::ZERO)
        .finish();

    HttpResponse::Found()
        .cookie(crate::session::make_cookie(&token))
        .cookie(clear_state)
        .append_header(("Location", "/"))
        .finish()
}

fn derive_username(db: &Database, userinfo: &crate::oidc::UserInfo) -> String {
    let base: String = userinfo
        .preferred_display_name()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let base = if base.is_empty() {
        "oidc_user".to_string()
    } else {
        base
    };

    if crate::db::get_user(db, &base).is_none() {
        return base;
    }
    for i in 2..=99 {
        let candidate = format!("{base}{i}");
        if crate::db::get_user(db, &candidate).is_none() {
            return candidate;
        }
    }
    format!("{base}_{}", &crate::session::generate_token()[..8])
}
