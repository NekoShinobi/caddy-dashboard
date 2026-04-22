use actix_web::{get, web, HttpResponse};
use futures_util::stream;
use redb::Database;
use serde::Deserialize;
use tokio::sync::broadcast;

#[derive(Deserialize)]
struct Query {
    page: Option<usize>,
    limit: Option<usize>,
    status: Option<u16>,
    host: Option<String>,
    method: Option<String>,
}

#[get("/logs")]
async fn get_logs(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let mut entries = crate::db::load_entries(&db);

    if let Some(status) = query.status {
        entries.retain(|e| e.status == status);
    }
    if let Some(ref host) = query.host {
        entries.retain(|e| e.request.host.contains(host.as_str()));
    }
    if let Some(ref method) = query.method {
        entries.retain(|e| e.request.method.eq_ignore_ascii_case(method));
    }

    entries.sort_unstable_by(|a, b| b.ts.partial_cmp(&a.ts).unwrap_or(std::cmp::Ordering::Equal));

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

#[get("/logs/stream")]
async fn stream_logs(tx: web::Data<broadcast::Sender<crate::log_parser::LogEntry>>) -> HttpResponse {
    let rx = tx.subscribe();

    let event_stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(entry) => {
                let json = serde_json::to_string(&entry).unwrap_or_default();
                Some((
                    Ok::<actix_web::web::Bytes, actix_web::Error>(
                        actix_web::web::Bytes::from(format!("data: {json}\n\n")),
                    ),
                    rx,
                ))
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Ok(actix_web::web::Bytes::from_static(b": lagged\n\n")), rx))
            }
            Err(_) => None,
        }
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(event_stream)
}
