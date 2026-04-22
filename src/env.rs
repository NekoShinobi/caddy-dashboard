use std::sync::LazyLock;

pub static LOG_PATH: LazyLock<String> =
    LazyLock::new(|| std::env::var("LOG_PATH").unwrap_or_else(|_| "/config/access.log".to_string()));

pub static DATA_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string()));

pub static PORT: LazyLock<u16> = LazyLock::new(|| {
    std::env::var("PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse()
        .expect("PORT must be a valid u16")
});
