use actix_web::{
    cookie::{time::Duration, Cookie, SameSite},
    HttpRequest,
};
use rand::{rngs::OsRng, RngCore};

pub const COOKIE_NAME: &str = "cd_session";

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        write!(s, "{b:02x}").unwrap();
        s
    })
}

pub fn make_cookie(token: &str) -> Cookie<'static> {
    Cookie::build(COOKIE_NAME, token.to_owned())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(*crate::env::COOKIE_SECURE)
        .max_age(Duration::days(30))
        .finish()
}

pub fn clear_cookie() -> Cookie<'static> {
    Cookie::build(COOKIE_NAME, "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(Duration::ZERO)
        .finish()
}

pub fn get_token(req: &HttpRequest) -> Option<String> {
    req.cookie(COOKIE_NAME).map(|c| c.value().to_string())
}

pub fn get_username(req: &HttpRequest, db: &redb::Database) -> Option<String> {
    crate::db::get_session(db, &get_token(req)?)
}
