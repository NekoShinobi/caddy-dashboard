use std::sync::LazyLock;

pub static LOG_PATH: LazyLock<String> =
    LazyLock::new(|| std::env::var("LOG_PATH").unwrap_or_else(|_| "/config/access.log".to_string()));

pub static DATA_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string()));

pub static GEOIP_DB: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("GEOIP_DB").ok());

/// 0 means retention is disabled.
pub static RETENTION_DAYS: LazyLock<u64> = LazyLock::new(|| {
    std::env::var("RETENTION_DAYS")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("RETENTION_DAYS must be a non-negative integer")
});

pub static OLLAMA_HOST: LazyLock<String> = LazyLock::new(|| {
    std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
});

pub static OLLAMA_MODEL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string())
});

/// Set to "false" to allow session cookies over plain HTTP (e.g. local dev behind a proxy).
/// Defaults to true; always enable in production.
pub static COOKIE_SECURE: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("COOKIE_SECURE")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true)
});

pub static PORT: LazyLock<u16> = LazyLock::new(|| {
    std::env::var("PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse()
        .expect("PORT must be a valid u16")
});

/// Base URL used to construct the OIDC redirect URI (e.g. "https://dash.example.com").
/// Trailing slash is stripped. Required when behind a reverse proxy.
pub static BASE_URL: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("BASE_URL").ok()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
});

// ── OIDC ────────────────────────────────────────────────────────────────────

/// When set (non-empty), OIDC login is enabled.
pub static OIDC_CLIENT_ID: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("OIDC_CLIENT_ID").ok().filter(|s| !s.is_empty()));

pub static OIDC_CLIENT_SECRET: LazyLock<String> =
    LazyLock::new(|| std::env::var("OIDC_CLIENT_SECRET").unwrap_or_default());

/// Trailing slashes are stripped so discovery URL construction is consistent.
pub static OIDC_ISSUER_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("OIDC_ISSUER_URL")
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string()
});

pub static OIDC_SCOPES: LazyLock<String> =
    LazyLock::new(|| std::env::var("OIDC_SCOPES").unwrap_or_else(|_| "openid email profile".to_string()));

/// Claim name to inspect for admin rights (e.g. "groups", "roles").
pub static OIDC_ADMIN_CLAIM: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("OIDC_ADMIN_CLAIM").ok().filter(|s| !s.is_empty()));

/// Value within OIDC_ADMIN_CLAIM that grants admin (e.g. "admin").
pub static OIDC_ADMIN_VALUE: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("OIDC_ADMIN_VALUE").ok().filter(|s| !s.is_empty()));

pub static OIDC_PROVIDER_NAME: LazyLock<String> =
    LazyLock::new(|| std::env::var("OIDC_PROVIDERS_NAME").unwrap_or_else(|_| "SSO".to_string()));

pub static OIDC_PROVIDER_LOGO_URL: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("OIDC_PROVIDER_LOGO_URL").ok().filter(|s| !s.is_empty()));

/// When true, the local username/password login form is hidden and the endpoint returns 403.
pub static OIDC_DISABLE_LOGIN: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("OIDC_DISABLE_LOGIN")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
});
