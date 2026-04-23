use std::sync::LazyLock;

pub static LOG_PATH: LazyLock<String> =
    LazyLock::new(|| std::env::var("UTh").unwrap_or_else(|_| "/config/access.log".to_string()));

pub static DATA_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string()));

/// 0 means retention is disabled.
pub static RETENTION_DAYS: LazyLock<u64> = LazyLock::new(|| {
    std::env::var("RETENTION_DAYS")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("RETENTION_DAYS must be a non-negative integer")
});

pub static PORT: LazyLock<u16> = LazyLock::new(|| {
    std::env::var("PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse()
        .expect("PORT must be a valid u16")
});
