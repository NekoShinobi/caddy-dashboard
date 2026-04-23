use actix_web::{get, web, HttpResponse};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct Query {
    since: Option<f64>,
    mode: Option<String>,
}

#[derive(Serialize)]
struct CountryCount {
    country: String,
    count: usize,
}

#[derive(Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum GeoResponse {
    Country { data: Vec<CountryCount> },
    Precise { points: Vec<[f64; 3]> },
}

#[get("/geo")]
async fn get_geo(
    db: web::Data<Database>,
    geoip: web::Data<crate::geoip::GeoIpDb>,
    query: web::Query<Query>,
) -> HttpResponse {
    let all = crate::db::load_entries(&db);
    let entries: Vec<_> = match query.since {
        Some(since) => all.into_iter().filter(|e| e.ts >= since).collect(),
        None => all,
    };

    let use_precise = query.mode.as_deref() == Some("precise");

    if use_precise && geoip.is_some() {
        // Aggregate to ~1 km grid (2 decimal places ≈ 1.1 km)
        let mut grid: HashMap<(i64, i64), f64> = HashMap::new();
        for e in &entries {
            let ip_str = if e.request.client_ip.is_empty() {
                &e.request.remote_ip
            } else {
                &e.request.client_ip
            };
            let Ok(ip) = ip_str.parse::<std::net::IpAddr>() else { continue };
            let Some((lat, lng)) = geoip.lookup_city(ip) else { continue };
            let key = ((lat * 100.0).round() as i64, (lng * 100.0).round() as i64);
            *grid.entry(key).or_insert(0.0) += 1.0;
        }
        let points: Vec<[f64; 3]> = grid
            .into_iter()
            .map(|((lat, lng), w)| [lat as f64 / 100.0, lng as f64 / 100.0, w])
            .collect();
        return HttpResponse::Ok().json(GeoResponse::Precise { points });
    }

    // Country mode (default or MMDB unavailable)
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in &entries {
        if let Some(codes) = e.request.headers.get("Cf-Ipcountry") {
            if let Some(code) = codes.first() {
                *counts.entry(code.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut data: Vec<CountryCount> = counts
        .into_iter()
        .map(|(country, count)| CountryCount { country, count })
        .collect();
    data.sort_unstable_by(|a, b| b.count.cmp(&a.count));
    HttpResponse::Ok().json(GeoResponse::Country { data })
}
