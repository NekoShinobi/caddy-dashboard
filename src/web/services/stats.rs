use crate::analytics::Rollup;
use actix_web::{HttpResponse, get, web};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Deserialize)]
struct Query {
    since: Option<f64>,
    range: Option<u64>,
}

#[derive(Clone, Serialize)]
struct SlowPath {
    path: String,
    avg_ms: f64,
    p99_ms: f64,
    count: usize,
}

#[derive(Clone, Serialize)]
struct Stats {
    total_requests: usize,
    status_codes: HashMap<u16, usize>,
    top_paths: Vec<(String, usize)>,
    top_hosts: Vec<(String, usize)>,
    top_ips: Vec<(String, usize)>,
    avg_duration_ms: f64,
    total_bytes: u64,
    unique_clients: usize,
    top_referrers: Vec<(String, usize)>,
    top_user_agents: Vec<(String, usize)>,
    slowest_paths: Vec<SlowPath>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct CacheKey {
    range: Option<u64>,
    legacy_since_bits: Option<u64>,
    window_tick: u64,
    generation: u64,
}

static CACHE: LazyLock<Mutex<HashMap<CacheKey, Stats>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn top(mut values: HashMap<String, u64>, limit: usize) -> Vec<(String, usize)> {
    let mut values: Vec<_> = values
        .drain()
        .map(|(key, count)| (key, count as usize))
        .collect();
    values.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    values.truncate(limit);
    values
}

fn stats_from_rollup(rollup: Rollup) -> Stats {
    let unique_clients = rollup.unique_clients();
    let mut slowest_paths: Vec<SlowPath> = rollup
        .paths
        .iter()
        .filter(|(_, value)| value.count >= 2)
        .map(|(path, value)| SlowPath {
            path: path.clone(),
            avg_ms: value.duration_sum_ms / value.count as f64,
            p99_ms: value.durations.quantile(99.0),
            count: value.count as usize,
        })
        .collect();
    slowest_paths.sort_unstable_by(|a, b| {
        b.p99_ms
            .partial_cmp(&a.p99_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    slowest_paths.truncate(10);

    Stats {
        total_requests: rollup.total as usize,
        status_codes: rollup
            .status_codes
            .into_iter()
            .map(|(code, count)| (code, count as usize))
            .collect(),
        top_paths: top(
            rollup
                .paths
                .iter()
                .map(|(path, value)| (path.clone(), value.count))
                .collect(),
            10,
        ),
        top_hosts: top(rollup.hosts, 10),
        top_ips: top(rollup.ips, 10),
        avg_duration_ms: if rollup.total > 0 {
            rollup.duration_sum_ms / rollup.total as f64
        } else {
            0.0
        },
        total_bytes: rollup.total_bytes,
        unique_clients,
        top_referrers: top(rollup.referrers, 10),
        top_user_agents: top(rollup.user_agents, 10),
        slowest_paths,
    }
}

#[get("/stats")]
async fn get_stats(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let started = Instant::now();
    let query = query.into_inner();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let since = match query.range {
        Some(0) => None,
        Some(seconds) => Some(now - seconds as f64),
        None => query.since,
    };
    let db = db.into_inner();
    let result = web::block(move || {
        let generation = crate::db::analytics_generation(&db);
        let key = CacheKey {
            range: query.range,
            legacy_since_bits: query
                .range
                .is_none()
                .then(|| query.since.map(f64::to_bits))
                .flatten(),
            window_tick: query
                .range
                .filter(|seconds| *seconds > 0)
                .map(|_| now as u64 / 30)
                .unwrap_or(0),
            generation,
        };
        if let Some(cached) = CACHE.lock().unwrap().get(&key).cloned() {
            return Ok::<_, String>((cached, true));
        }

        let stats = stats_from_rollup(crate::db::aggregate_range(&db, since, now)?);
        let mut cache = CACHE.lock().unwrap();
        if cache.len() >= 64 {
            cache.clear();
        }
        cache.insert(key, stats.clone());
        Ok((stats, false))
    })
    .await;

    let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
    match result {
        Ok(Ok((stats, cache_hit))) => HttpResponse::Ok()
            .insert_header((
                "Server-Timing",
                format!(
                    "stats;dur={elapsed_ms:.1}, cache;desc=\"{}\"",
                    if cache_hit { "hit" } else { "miss" }
                ),
            ))
            .json(stats),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("stats blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}
