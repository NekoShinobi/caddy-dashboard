use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};

#[derive(Debug, Clone, Deserialize)]
pub struct OidcDiscovery {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    /// Present on providers that support RP-initiated logout (RFC 9207 / OpenID RP-Initiated Logout)
    pub end_session_endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl UserInfo {
    pub fn preferred_display_name(&self) -> String {
        if let Some(p) = &self.preferred_username {
            if !p.is_empty() {
                return p.clone();
            }
        }
        if let Some(e) = &self.email {
            let local = e.split('@').next().unwrap_or(e);
            if !local.is_empty() {
                return local.to_string();
            }
        }
        self.sub.clone()
    }

    pub fn check_admin(&self) -> bool {
        let Some(claim) = crate::env::OIDC_ADMIN_CLAIM.as_deref() else {
            return false;
        };
        let Some(want) = crate::env::OIDC_ADMIN_VALUE.as_deref() else {
            return false;
        };

        // Check named standard fields first
        let std_match = match claim {
            "sub" => self.sub == want,
            "email" => self.email.as_deref() == Some(want),
            "name" => self.name.as_deref() == Some(want),
            "preferred_username" => self.preferred_username.as_deref() == Some(want),
            _ => false,
        };
        if std_match {
            return true;
        }

        // Check extra / provider-specific claims
        match self.extra.get(claim) {
            Some(serde_json::Value::String(s)) => s == want,
            Some(serde_json::Value::Array(arr)) => {
                arr.iter().any(|v| v.as_str() == Some(want))
            }
            Some(serde_json::Value::Bool(b)) => want == (if *b { "true" } else { "false" }),
            _ => false,
        }
    }
}

pub fn is_enabled() -> bool {
    crate::env::OIDC_CLIENT_ID.is_some()
}

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
static DISCOVERY: OnceLock<OidcDiscovery> = OnceLock::new();

pub async fn fetch_discovery() -> Result<OidcDiscovery, String> {
    if let Some(d) = DISCOVERY.get() {
        return Ok(d.clone());
    }
    let issuer = crate::env::OIDC_ISSUER_URL.as_str();
    if issuer.is_empty() {
        return Err("OIDC_ISSUER_URL is not set".to_string());
    }
    let url = format!("{issuer}/.well-known/openid-configuration");
    let resp = HTTP
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("OIDC discovery fetch failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OIDC discovery returned {status}: {body}"));
    }
    let disc: OidcDiscovery = resp
        .json()
        .await
        .map_err(|e| format!("OIDC discovery parse failed: {e}"))?;
    let _ = DISCOVERY.set(disc.clone());
    Ok(DISCOVERY.get().unwrap().clone())
}

pub async fn exchange_code(
    disc: &OidcDiscovery,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, String> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", crate::env::OIDC_CLIENT_ID.as_deref().unwrap_or("")),
        ("client_secret", crate::env::OIDC_CLIENT_SECRET.as_str()),
    ];
    let resp = HTTP
        .post(&disc.token_endpoint)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("token exchange request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("token endpoint returned {status}: {body}"));
    }
    resp.json()
        .await
        .map_err(|e| format!("token response parse failed: {e}"))
}

pub async fn get_userinfo(disc: &OidcDiscovery, access_token: &str) -> Result<UserInfo, String> {
    let resp = HTTP
        .get(&disc.userinfo_endpoint)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("userinfo request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("userinfo returned {status}: {body}"));
    }
    resp.json()
        .await
        .map_err(|e| format!("userinfo parse failed: {e}"))
}
