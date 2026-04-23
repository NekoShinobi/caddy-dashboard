use actix_web::{get, web, HttpResponse};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
struct Query {
    bucket: Option<String>,
}

#[derive(Serialize)]
struct Bucket {
    ts: u64,
    total: usize,
    s2xx: usize,
    s3xx: usize,
    s4xx: usize,
    s5xx: usize,
    avg_duration_ms: f64,
    median_duration_ms: f64,
    p99_duration_ms: f64,
    avg_size: f64,
    median_size: f64,
    p99_size: f64,
    unique_clients: usize,
    methods: HashMap<String, usize>,
}

fn percentile_f64(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).ceil() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn percentile_u64(sorted: &[u64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).ceil() as usize;
    sorted[idx.min(sorted.len() - 1)] as f64
}

struct BucketAccum {
    total: usize,
    s2xx: usize,
    s3xx: usize,
    s4xx: usize,
    s5xx: usize,
    durations: Vec<f64>,
    sizes: Vec<u64>,
    hosts: HashSet<String>,
    methods: HashMap<String, usize>,
}

impl BucketAccum {
    fn new() -> Self {
        Self { total: 0, s2xx: 0, s3xx: 0, s4xx: 0, s5xx: 0,
               durations: Vec::new(), sizes: Vec::new(), hosts: HashSet::new(),
               methods: HashMap::new() }
    }
}

#[get("/timeline")]
async fn get_timeline(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let entries = crate::db::load_entries(&db);

    let bucket_secs: u64 = match query.bucket.as_deref().unwrap_or("hour") {
        "minute" => 60,
        "day" => 86400,
        _ => 3600,
    };

    let mut map: HashMap<u64, BucketAccum> = HashMap::new();

    for e in &entries {
        let key = (e.ts as u64 / bucket_secs) * bucket_secs;
        let b = map.entry(key).or_insert_with(BucketAccum::new);
        b.total += 1;
        match e.status {
            200..=299 => b.s2xx += 1,
            300..=399 => b.s3xx += 1,
            400..=499 => b.s4xx += 1,
            _ => b.s5xx += 1,
        }
        b.durations.push(e.duration * 1000.0);
        b.sizes.push(e.size);
        b.hosts.insert(e.request.client_ip.clone());
        *b.methods.entry(e.request.method.clone()).or_insert(0) += 1;
    }

    let mut buckets: Vec<Bucket> = map
        .into_iter()
        .map(|(ts, mut b)| {
            b.durations.sort_unstable_by(|a, c| a.partial_cmp(c).unwrap());
            b.sizes.sort_unstable();
            let avg_duration_ms = if b.total > 0 {
                b.durations.iter().sum::<f64>() / b.total as f64
            } else { 0.0 };
            let avg_size = if b.total > 0 {
                b.sizes.iter().sum::<u64>() as f64 / b.total as f64
            } else { 0.0 };
            Bucket {
                ts,
                total: b.total,
                s2xx: b.s2xx,
                s3xx: b.s3xx,
                s4xx: b.s4xx,
                s5xx: b.s5xx,
                avg_duration_ms,
                median_duration_ms: percentile_f64(&b.durations, 50.0),
                p99_duration_ms: percentile_f64(&b.durations, 99.0),
                avg_size,
                median_size: percentile_u64(&b.sizes, 50.0),
                p99_size: percentile_u64(&b.sizes, 99.0),
                unique_clients: b.hosts.len(),
                methods: b.methods,
            }
        })
        .collect();

    buckets.sort_unstable_by_key(|b| b.ts);

    HttpResponse::Ok().json(serde_json::json!({ "buckets": buckets, "bucket_secs": bucket_secs }))
}
