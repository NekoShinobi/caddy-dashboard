use actix_web::{get, web, HttpResponse};
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

#[get("/reports/error-rates")]
pub async fn get_error_rates(
    db: web::Data<Database>,
    query: web::Query<Query>,
) -> HttpResponse {
    let min_requests = query.min_requests.unwrap_or(5);
    let all = crate::db::load_entries(&db);
    let entries: Vec<_> = match query.since {
        Some(since) => all.into_iter().filter(|e| e.ts >= since).collect(),
        None => all,
    };

    struct IpAccum {
        total: usize,
        errors_4xx: usize,
        errors_5xx: usize,
        // (method, path) → status_code → count
        endpoints: HashMap<(String, String), HashMap<u16, usize>>,
    }

    let mut by_ip: HashMap<String, IpAccum> = HashMap::new();

    for e in &entries {
        let ip = if e.request.client_ip.is_empty() {
            e.request.remote_ip.clone()
        } else {
            e.request.client_ip.clone()
        };

        let acc = by_ip.entry(ip).or_insert(IpAccum {
            total: 0,
            errors_4xx: 0,
            errors_5xx: 0,
            endpoints: HashMap::new(),
        });

        acc.total += 1;

        if e.status >= 400 && e.status < 500 {
            acc.errors_4xx += 1;
            *acc.endpoints
                .entry((e.request.method.clone(), e.request.uri.clone()))
                .or_default()
                .entry(e.status)
                .or_insert(0) += 1;
        } else if e.status >= 500 {
            acc.errors_5xx += 1;
            *acc.endpoints
                .entry((e.request.method.clone(), e.request.uri.clone()))
                .or_default()
                .entry(e.status)
                .or_insert(0) += 1;
        }
    }

    let mut reports: Vec<IpReport> = by_ip
        .into_iter()
        .filter(|(_, acc)| {
            acc.total >= min_requests && (acc.errors_4xx + acc.errors_5xx) > 0
        })
        .map(|(ip, acc)| {
            let mut endpoints: Vec<EndpointStat> = acc
                .endpoints
                .into_iter()
                .map(|((method, path), codes)| {
                    let errors: usize = codes.values().sum();
                    let mut codes_vec: Vec<(u16, usize)> = codes.into_iter().collect();
                    codes_vec.sort_unstable_by(|a, b| b.1.cmp(&a.1));
                    EndpointStat { method, path, errors, codes: codes_vec }
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

    // Sort by total errors descending
    reports.sort_unstable_by(|a, b| {
        (b.errors_4xx + b.errors_5xx).cmp(&(a.errors_4xx + a.errors_5xx))
    });

    HttpResponse::Ok().json(reports)
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

#[get("/reports/large-payloads")]
pub async fn get_large_payloads(
    db: web::Data<Database>,
    query: web::Query<Query>,
) -> HttpResponse {
    let all = crate::db::load_entries(&db);
    let mut entries: Vec<_> = match query.since {
        Some(since) => all.into_iter().filter(|e| e.ts >= since).collect(),
        None => all,
    };

    entries.sort_unstable_by(|a, b| b.size.cmp(&a.size));
    entries.truncate(100);

    let payloads: Vec<LargePayload> = entries
        .into_iter()
        .map(|e| LargePayload {
            ts: e.ts,
            method: e.request.method,
            host: e.request.host,
            uri: e.request.uri,
            status: e.status,
            size: e.size,
            ip: if e.request.client_ip.is_empty() { e.request.remote_ip } else { e.request.client_ip },
            duration: e.duration,
        })
        .collect();

    HttpResponse::Ok().json(payloads)
}
