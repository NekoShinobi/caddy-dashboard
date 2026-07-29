use crate::analytics::Rollup;
use actix_web::{HttpResponse, get, web};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Deserialize)]
struct Query {
    bucket: Option<String>,
}

#[derive(Clone, Serialize)]
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

#[derive(Clone, Serialize)]
struct Timeline {
    buckets: Vec<Bucket>,
    bucket_secs: u64,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct CacheKey {
    bucket_secs: u64,
    end_bucket: u64,
    generation: u64,
}

static CACHE: LazyLock<Mutex<HashMap<CacheKey, Timeline>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn status_count(rollup: &Rollup, range: std::ops::RangeInclusive<u16>) -> usize {
    rollup
        .status_codes
        .iter()
        .filter(|(status, _)| range.contains(status))
        .map(|(_, count)| *count as usize)
        .sum()
}

fn make_bucket(ts: u64, rollup: Rollup) -> Bucket {
    Bucket {
        ts,
        total: rollup.total as usize,
        s2xx: status_count(&rollup, 200..=299),
        s3xx: status_count(&rollup, 300..=399),
        s4xx: status_count(&rollup, 400..=499),
        s5xx: status_count(&rollup, 500..=u16::MAX),
        avg_duration_ms: if rollup.total > 0 {
            rollup.duration_sum_ms / rollup.total as f64
        } else {
            0.0
        },
        median_duration_ms: rollup.durations.quantile(50.0),
        p99_duration_ms: rollup.durations.quantile(99.0),
        avg_size: if rollup.total > 0 {
            rollup.total_bytes as f64 / rollup.total as f64
        } else {
            0.0
        },
        median_size: rollup.sizes.quantile(50.0),
        p99_size: rollup.sizes.quantile(99.0),
        unique_clients: rollup.unique_clients(),
        methods: rollup
            .methods
            .into_iter()
            .map(|(method, count)| (method, count as usize))
            .collect(),
    }
}

fn build_timeline(
    db: &Database,
    bucket_secs: u64,
    window_secs: u64,
    now: u64,
) -> Result<Timeline, String> {
    let cutoff = now.saturating_sub(window_secs) as f64;
    let start_bucket = (cutoff as u64 / bucket_secs) * bucket_secs;
    let end_bucket = (now / bucket_secs) * bucket_secs;
    let mut buckets = Vec::new();
    let mut timestamp = start_bucket;
    while timestamp <= end_bucket {
        let rollup =
            crate::db::aggregate_timeline_bucket(db, timestamp, bucket_secs, cutoff, now as f64)?;
        buckets.push(make_bucket(timestamp, rollup));
        timestamp += bucket_secs;
    }
    Ok(Timeline {
        buckets,
        bucket_secs,
    })
}

#[get("/timeline")]
async fn get_timeline(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let bucket = query.bucket.as_deref().unwrap_or("hour");
    let (bucket_secs, window_secs): (u64, u64) = match bucket {
        "minute" => (60, 3_600),
        "hour" => (3_600, 86_400),
        "day" => (86_400, 2_592_000),
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid bucket. Valid values: minute, hour, day"
            }));
        }
    };

    let started = Instant::now();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let db = db.into_inner();
    let result = web::block(move || {
        let key = CacheKey {
            bucket_secs,
            end_bucket: (now / bucket_secs) * bucket_secs,
            generation: crate::db::analytics_generation(&db),
        };
        if let Some(cached) = CACHE.lock().unwrap().get(&key).cloned() {
            return Ok::<_, String>((cached, true));
        }
        let timeline = build_timeline(&db, bucket_secs, window_secs, now)?;
        let mut cache = CACHE.lock().unwrap();
        if cache.len() >= 16 {
            cache.clear();
        }
        cache.insert(key, timeline.clone());
        Ok((timeline, false))
    })
    .await;

    let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
    match result {
        Ok(Ok((timeline, cache_hit))) => HttpResponse::Ok()
            .insert_header((
                "Server-Timing",
                format!(
                    "timeline;dur={elapsed_ms:.1}, cache;desc=\"{}\"",
                    if cache_hit { "hit" } else { "miss" }
                ),
            ))
            .json(timeline),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("timeline blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}
