use actix_web::{HttpResponse, get, web};
use futures_util::stream;
use redb::Database;
use serde::Deserialize;
use std::time::Instant;
use tokio::sync::broadcast;

/// Match `text` against `pattern` where `*` matches any sequence of characters.
/// Without `*` the match is exact (case-sensitive).
fn glob_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return text == pattern;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let search = &text[pos..];
        if i == 0 {
            // first segment must match at current position
            if !search.starts_with(part) {
                return false;
            }
            pos += part.len();
        } else {
            match search.find(part) {
                Some(idx) => pos += idx + part.len(),
                None => return false,
            }
        }
    }
    // if pattern doesn't end with *, last segment must reach end
    if !pattern.ends_with('*') {
        if let Some(last) = parts.last() {
            if !last.is_empty() && !text.ends_with(last) {
                return false;
            }
        }
    }
    true
}

fn match_status(filter: &str, code: u16) -> bool {
    filter.split(',').any(|s| {
        let s = s.trim();
        if s.len() == 3 && s.ends_with("xx") {
            if let Some(d) = s.chars().next().and_then(|c| c.to_digit(10)) {
                return code / 100 == d as u16;
            }
        }
        s.parse::<u16>().map(|n| code == n).unwrap_or(false)
    })
}

#[derive(Clone, Deserialize)]
struct Query {
    page: Option<usize>,
    limit: Option<usize>,
    cursor: Option<u64>,
    status: Option<String>,
    host: Option<String>,
    method: Option<String>,
    ip: Option<String>,
    path: Option<String>,
    ua: Option<String>,
    duration_gt: Option<f64>,
    size_gt: Option<u64>,
    size_lt: Option<u64>,
    text: Option<String>,
    // negation filters
    not_status: Option<String>,
    not_host: Option<String>,
    not_method: Option<String>,
    not_ip: Option<String>,
    not_path: Option<String>,
}

fn entry_matches(entry: &crate::log_parser::LogEntry, query: &Query) -> bool {
    if let Some(ref status) = query.status {
        if !match_status(status, entry.status) {
            return false;
        }
    }
    if let Some(ref host) = query.host {
        if !entry.request.host.contains(host.as_str()) {
            return false;
        }
    }
    if let Some(ref method) = query.method {
        if !entry.request.method.eq_ignore_ascii_case(method) {
            return false;
        }
    }
    if let Some(ref ip) = query.ip {
        if !entry.request.client_ip.contains(ip.as_str())
            && !entry.request.remote_ip.contains(ip.as_str())
        {
            return false;
        }
    }
    if let Some(ref path) = query.path {
        if !glob_match(path, &entry.request.uri) {
            return false;
        }
    }
    if let Some(ref ua) = query.ua {
        let ua_lower = ua.to_lowercase();
        let matches = entry
            .request
            .headers
            .get("User-Agent")
            .and_then(|values| values.first())
            .map(|value| value.to_lowercase().contains(&ua_lower))
            .unwrap_or(false);
        if !matches {
            return false;
        }
    }
    if let Some(gt) = query.duration_gt {
        if entry.duration < gt / 1000.0 {
            return false;
        }
    }
    if let Some(gt) = query.size_gt {
        if entry.size <= gt {
            return false;
        }
    }
    if let Some(lt) = query.size_lt {
        if entry.size >= lt {
            return false;
        }
    }
    if let Some(ref text) = query.text {
        let t = text.to_lowercase();
        if !entry.request.uri.to_lowercase().contains(&t)
            && !entry.request.host.to_lowercase().contains(&t)
            && !entry.request.client_ip.contains(t.as_str())
        {
            return false;
        }
    }
    if let Some(ref s) = query.not_status {
        if s.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|value| match_status(value, entry.status))
        {
            return false;
        }
    }
    if let Some(ref h) = query.not_host {
        if h.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|value| entry.request.host.contains(value))
        {
            return false;
        }
    }
    if let Some(ref m) = query.not_method {
        if m.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|value| entry.request.method.eq_ignore_ascii_case(value))
        {
            return false;
        }
    }
    if let Some(ref ip) = query.not_ip {
        if ip
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|value| {
                entry.request.client_ip.contains(value) || entry.request.remote_ip.contains(value)
            })
        {
            return false;
        }
    }
    if let Some(ref p) = query.not_path {
        if p.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .any(|value| glob_match(value, &entry.request.uri))
        {
            return false;
        }
    }
    true
}

fn has_filters(query: &Query) -> bool {
    query.status.is_some()
        || query.host.is_some()
        || query.method.is_some()
        || query.ip.is_some()
        || query.path.is_some()
        || query.ua.is_some()
        || query.duration_gt.is_some()
        || query.size_gt.is_some()
        || query.size_lt.is_some()
        || query.text.is_some()
        || query.not_status.is_some()
        || query.not_host.is_some()
        || query.not_method.is_some()
        || query.not_ip.is_some()
        || query.not_path.is_some()
}

fn scan_source(query: &Query) -> crate::db::LogScan {
    if let Some(status) = query.status.as_deref().and_then(|value| value.parse().ok()) {
        crate::db::LogScan::Status(status)
    } else if let Some(method) = &query.method {
        crate::db::LogScan::Method(method.to_ascii_uppercase())
    } else {
        crate::db::LogScan::All
    }
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

#[get("/logs")]
async fn get_logs(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let started = Instant::now();
    let query = query.into_inner();
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let page = query.page.unwrap_or(0);
    let cursor = query.cursor;
    let skip = if cursor.is_none() {
        page.saturating_mul(limit)
    } else {
        0
    };
    let source = scan_source(&query);
    let include_total = !has_filters(&query);
    let db = db.into_inner();
    let result = web::block(move || {
        crate::db::scan_logs_page(&db, source, cursor, skip, limit, include_total, |entry| {
            entry_matches(entry, &query)
        })
    })
    .await;

    let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
    match result {
        Ok(Ok(result)) => HttpResponse::Ok()
            .insert_header((
                "Server-Timing",
                format!(
                    "logs;dur={elapsed_ms:.1};desc=\"scanned {}\"",
                    result.scanned
                ),
            ))
            .json(serde_json::json!({
                "total": result.total,
                "page": page,
                "limit": limit,
                "next_cursor": result.next_cursor.map(|cursor| cursor.to_string()),
                "has_more": result.has_more,
                "entries": result.entries,
            })),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("logs blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}

#[get("/logs/export")]
async fn export_logs_csv(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let query = query.into_inner();
    let db = db.into_inner();
    let result = web::block(move || {
        let mut csv = String::from(
            "timestamp,unix_ts,status,method,protocol,host,path,duration_ms,size_bytes,bytes_read,client_ip,remote_ip,user_agent\n"
        );
        crate::db::visit_logs_newest(&db, |entry| {
            if !entry_matches(entry, &query) {
                return;
            }
            let ts = chrono::DateTime::from_timestamp(entry.ts as i64, 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_default();
            let ua = entry.request.headers.get("User-Agent")
                .and_then(|values| values.first())
                .map(String::as_str)
                .unwrap_or("");
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{:.3},{},{},{},{},{}\n",
                csv_field(&ts), entry.ts, entry.status,
                csv_field(&entry.request.method), csv_field(&entry.request.proto),
                csv_field(&entry.request.host), csv_field(&entry.request.uri),
                entry.duration * 1000.0, entry.size, entry.bytes_read,
                csv_field(&entry.request.client_ip), csv_field(&entry.request.remote_ip),
                csv_field(ua),
            ));
        })?;
        Ok::<_, String>(csv)
    })
    .await;

    match result {
        Ok(Ok(csv)) => HttpResponse::Ok()
            .insert_header(("Content-Type", "text/csv; charset=utf-8"))
            .insert_header((
                "Content-Disposition",
                "attachment; filename=\"caddy-logs.csv\"",
            ))
            .body(csv),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("CSV export blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}

#[get("/logs/stream")]
async fn stream_logs(
    tx: web::Data<broadcast::Sender<crate::log_parser::LogEntry>>,
) -> HttpResponse {
    let rx = tx.subscribe();

    let event_stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(entry) => match serde_json::to_string(&entry) {
                Ok(json) => Some((
                    Ok::<actix_web::web::Bytes, actix_web::Error>(actix_web::web::Bytes::from(
                        format!("data: {json}\n\n"),
                    )),
                    rx,
                )),
                Err(e) => {
                    log::error!("stream_logs: serialize entry: {e}");
                    None
                }
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                log::warn!("stream_logs: subscriber lagged, dropped {n} messages");
                Some((Ok(actix_web::web::Bytes::from_static(b": lagged\n\n")), rx))
            }
            Err(e) => {
                log::error!("stream_logs: broadcast channel closed: {e}");
                None
            }
        }
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(event_stream)
}
