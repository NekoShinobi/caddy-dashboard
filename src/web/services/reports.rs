use actix_web::{HttpResponse, get, web};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct Query {
    since: Option<f64>,
    min_requests: Option<usize>,
}

#[derive(Serialize)]
struct EndpointStat {
    method: String,
    path: String,
    errors: usize,
    codes: Vec<(u16, usize)>,
}

#[derive(Serialize)]
struct IpReport {
    ip: String,
    total: usize,
    errors_4xx: usize,
    errors_5xx: usize,
    top_endpoints: Vec<EndpointStat>,
}

struct IpAccum {
    total: usize,
    errors_4xx: usize,
    errors_5xx: usize,
    // (method, path) -> status_code -> count
    endpoints: HashMap<(String, String), HashMap<u16, usize>>,
}

fn build_error_rates(
    db: &Database,
    since: Option<f64>,
    min_requests: usize,
) -> Result<Vec<IpReport>, String> {
    let mut by_ip: HashMap<String, IpAccum> = HashMap::new();

    crate::db::visit_logs_since(db, since, |entry| {
        let ip = if entry.request.client_ip.is_empty() {
            &entry.request.remote_ip
        } else {
            &entry.request.client_ip
        };

        let acc = by_ip.entry(ip.to_owned()).or_insert_with(|| IpAccum {
            total: 0,
            errors_4xx: 0,
            errors_5xx: 0,
            endpoints: HashMap::new(),
        });

        acc.total += 1;
        if (400..500).contains(&entry.status) {
            acc.errors_4xx += 1;
        } else if entry.status >= 500 {
            acc.errors_5xx += 1;
        } else {
            return;
        }

        *acc.endpoints
            .entry((entry.request.method.clone(), entry.request.uri.clone()))
            .or_default()
            .entry(entry.status)
            .or_insert(0) += 1;
    })?;

    let mut reports: Vec<IpReport> = by_ip
        .into_iter()
        .filter(|(_, acc)| acc.total >= min_requests && (acc.errors_4xx + acc.errors_5xx) > 0)
        .map(|(ip, acc)| {
            let mut endpoints: Vec<EndpointStat> = acc
                .endpoints
                .into_iter()
                .map(|((method, path), codes)| {
                    let errors = codes.values().sum();
                    let mut codes: Vec<(u16, usize)> = codes.into_iter().collect();
                    codes.sort_unstable_by(|a, b| b.1.cmp(&a.1));
                    EndpointStat {
                        method,
                        path,
                        errors,
                        codes,
                    }
                })
                .collect();
            endpoints.sort_unstable_by(|a, b| b.errors.cmp(&a.errors));
            endpoints.truncate(5);
            IpReport {
                ip,
                total: acc.total,
                errors_4xx: acc.errors_4xx,
                errors_5xx: acc.errors_5xx,
                top_endpoints: endpoints,
            }
        })
        .collect();

    reports
        .sort_unstable_by(|a, b| (b.errors_4xx + b.errors_5xx).cmp(&(a.errors_4xx + a.errors_5xx)));
    Ok(reports)
}

#[get("/reports/error-rates")]
pub async fn get_error_rates(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let min_requests = query.min_requests.unwrap_or(5);
    let since = query.since;
    let db = db.into_inner();
    match web::block(move || build_error_rates(&db, since, min_requests)).await {
        Ok(Ok(reports)) => HttpResponse::Ok().json(reports),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("error rates blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}

#[derive(Serialize)]
struct LargePayload {
    ts: f64,
    method: String,
    host: String,
    uri: String,
    status: u16,
    size: u64,
    ip: String,
    duration: f64,
}

fn build_large_payloads(db: &Database, since: Option<f64>) -> Result<Vec<LargePayload>, String> {
    let mut payloads: Vec<LargePayload> = Vec::with_capacity(100);
    crate::db::visit_logs_since(db, since, |entry| {
        let insert_at = payloads.partition_point(|payload| payload.size >= entry.size);
        if insert_at >= 100 {
            return;
        }

        payloads.insert(
            insert_at,
            LargePayload {
                ts: entry.ts,
                method: entry.request.method.clone(),
                host: entry.request.host.clone(),
                uri: entry.request.uri.clone(),
                status: entry.status,
                size: entry.size,
                ip: if entry.request.client_ip.is_empty() {
                    entry.request.remote_ip.clone()
                } else {
                    entry.request.client_ip.clone()
                },
                duration: entry.duration,
            },
        );
        if payloads.len() > 100 {
            payloads.pop();
        }
    })?;
    Ok(payloads)
}

#[get("/reports/large-payloads")]
pub async fn get_large_payloads(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let since = query.since;
    let db = db.into_inner();
    match web::block(move || build_large_payloads(&db, since)).await {
        Ok(Ok(payloads)) => HttpResponse::Ok().json(payloads),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("large payloads blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}
