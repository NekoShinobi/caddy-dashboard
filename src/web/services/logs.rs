use actix_web::{get, web, HttpResponse};
use futures_util::stream;
use redb::Database;
use serde::Deserialize;
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

#[derive(Deserialize)]
struct Query {
    page: Option<usize>,
    limit: Option<usize>,
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

fn apply_filters(
    mut entries: Vec<crate::log_parser::LogEntry>,
    query: &Query,
) -> Vec<crate::log_parser::LogEntry> {
    if let Some(ref status) = query.status {
        entries.retain(|e| match_status(status, e.status));
    }
    if let Some(ref host) = query.host {
        entries.retain(|e| e.request.host.contains(host.as_str()));
    }
    if let Some(ref method) = query.method {
        entries.retain(|e| e.request.method.eq_ignore_ascii_case(method));
    }
    if let Some(ref ip) = query.ip {
        entries.retain(|e| e.request.client_ip.contains(ip.as_str()) || e.request.remote_ip.contains(ip.as_str()));
    }
    if let Some(ref path) = query.path {
        entries.retain(|e| glob_match(path, &e.request.uri));
    }
    if let Some(ref ua) = query.ua {
        let ua_lower = ua.to_lowercase();
        entries.retain(|e| {
            e.request.headers.get("User-Agent")
                .and_then(|v| v.first())
                .map(|v| v.to_lowercase().contains(&ua_lower))
                .unwrap_or(false)
        });
    }
    if let Some(gt) = query.duration_gt {
        entries.retain(|e| e.duration >= gt / 1000.0);
    }
    if let Some(gt) = query.size_gt {
        entries.retain(|e| e.size > gt);
    }
    if let Some(lt) = query.size_lt {
        entries.retain(|e| e.size < lt);
    }
    if let Some(ref text) = query.text {
        let t = text.to_lowercase();
        entries.retain(|e| {
            e.request.uri.to_lowercase().contains(&t)
                || e.request.host.to_lowercase().contains(&t)
                || e.request.client_ip.contains(t.as_str())
        });
    }
    if let Some(ref s) = query.not_status {
        entries.retain(|e| !match_status(s, e.status));
    }
    if let Some(ref h) = query.not_host {
        entries.retain(|e| !e.request.host.contains(h.as_str()));
    }
    if let Some(ref m) = query.not_method {
        entries.retain(|e| !e.request.method.eq_ignore_ascii_case(m));
    }
    if let Some(ref ip) = query.not_ip {
        entries.retain(|e| !e.request.client_ip.contains(ip.as_str()) && !e.request.remote_ip.contains(ip.as_str()));
    }
    if let Some(ref p) = query.not_path {
        entries.retain(|e| !glob_match(p, &e.request.uri));
    }
    entries.sort_unstable_by(|a, b| b.ts.partial_cmp(&a.ts).unwrap_or(std::cmp::Ordering::Equal));
    entries
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
    let raw = match crate::db::load_entries(&db) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e})),
    };
    let entries = apply_filters(raw, &query);
    let limit = query.limit.unwrap_or(50).min(500);
    let page = query.page.unwrap_or(0);
    let total = entries.len();
    let entries: Vec<_> = entries.into_iter().skip(page * limit).take(limit).collect();
    HttpResponse::Ok().json(serde_json::json!({
        "total": total,
        "page": page,
        "limit": limit,
        "entries": entries,
    }))
}

#[get("/logs/export")]
async fn export_logs_csv(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let raw = match crate::db::load_entries(&db) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e})),
    };
    let entries = apply_filters(raw, &query);

    let mut csv = String::from(
        "timestamp,unix_ts,status,method,protocol,host,path,duration_ms,size_bytes,bytes_read,client_ip,remote_ip,user_agent\n"
    );
    for e in &entries {
        let ts = chrono::DateTime::from_timestamp(e.ts as i64, 0)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();
        let ua = e.request.headers.get("User-Agent")
            .and_then(|v| v.first())
            .map(|s| s.as_str())
            .unwrap_or("");
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{:.3},{},{},{},{},{}\n",
            csv_field(&ts),
            e.ts,
            e.status,
            csv_field(&e.request.method),
            csv_field(&e.request.proto),
            csv_field(&e.request.host),
            csv_field(&e.request.uri),
            e.duration * 1000.0,
            e.size,
            e.bytes_read,
            csv_field(&e.request.client_ip),
            csv_field(&e.request.remote_ip),
            csv_field(ua),
        ));
    }

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/csv; charset=utf-8"))
        .insert_header(("Content-Disposition", "attachment; filename=\"caddy-logs.csv\""))
        .body(csv)
}

#[get("/logs/stream")]
async fn stream_logs(tx: web::Data<broadcast::Sender<crate::log_parser::LogEntry>>) -> HttpResponse {
    let rx = tx.subscribe();

    let event_stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(entry) => {
                match serde_json::to_string(&entry) {
                    Ok(json) => Some((
                        Ok::<actix_web::web::Bytes, actix_web::Error>(
                            actix_web::web::Bytes::from(format!("data: {json}\n\n")),
                        ),
                        rx,
                    )),
                    Err(e) => {
                        log::error!("stream_logs: serialize entry: {e}");
                        None
                    }
                }
            }
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
