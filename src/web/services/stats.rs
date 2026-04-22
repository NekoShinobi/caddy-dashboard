use actix_web::{get, web, HttpResponse};
use redb::Database;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct SlowPath {
    path: String,
    avg_ms: f64,
    p99_ms: f64,
    count: usize,
}

#[derive(Serialize)]
struct Stats {
    total_requests: usize,
    status_codes: HashMap<u16, usize>,
    top_paths: Vec<(String, usize)>,
    top_hosts: Vec<(String, usize)>,
    top_ips: Vec<(String, usize)>,
    avg_duration_ms: f64,
    total_bytes: u64,
    slowest_paths: Vec<SlowPath>,
}

#[get("/stats")]
async fn get_stats(db: web::Data<Database>) -> HttpResponse {
    let entries = crate::db::load_entries(&db);
    let total = entries.len();

    let mut status_codes: HashMap<u16, usize> = HashMap::new();
    let mut paths: HashMap<String, usize> = HashMap::new();
    let mut hosts: HashMap<String, usize> = HashMap::new();
    let mut ips: HashMap<String, usize> = HashMap::new();
    let mut path_durations: HashMap<String, Vec<f64>> = HashMap::new();
    let mut total_duration = 0.0f64;
    let mut total_bytes = 0u64;

    for e in &entries {
        *status_codes.entry(e.status).or_insert(0) += 1;
        let path_key = format!("{}{}", e.request.host, e.request.uri);
        *paths.entry(path_key.clone()).or_insert(0) += 1;
        path_durations.entry(path_key).or_default().push(e.duration * 1000.0);
        *hosts.entry(e.request.host.clone()).or_insert(0) += 1;
        *ips.entry(e.request.client_ip.clone()).or_insert(0) += 1;
        total_duration += e.duration;
        total_bytes += e.size;
    }

    let sort_top = |mut map: HashMap<String, usize>, n: usize| -> Vec<(String, usize)> {
        let mut v: Vec<_> = map.drain().collect();
        v.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    };

    let mut slowest_paths: Vec<SlowPath> = path_durations
        .into_iter()
        .filter(|(_, d)| d.len() >= 2)
        .map(|(path, mut durations)| {
            let count = durations.len();
            let avg_ms = durations.iter().sum::<f64>() / count as f64;
            durations.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            let p99_idx = ((0.99 * (count as f64 - 1.0)).ceil() as usize).min(count - 1);
            let p99_ms = durations[p99_idx];
            SlowPath { path, avg_ms, p99_ms, count }
        })
        .collect();
    slowest_paths.sort_unstable_by(|a, b| b.p99_ms.partial_cmp(&a.p99_ms).unwrap());
    slowest_paths.truncate(10);

    HttpResponse::Ok().json(Stats {
        total_requests: total,
        status_codes,
        top_paths: sort_top(paths, 10),
        top_hosts: sort_top(hosts, 10),
        top_ips: sort_top(ips, 10),
        avg_duration_ms: if total > 0 { total_duration / total as f64 * 1000.0 } else { 0.0 },
        total_bytes,
        slowest_paths,
    })
}
