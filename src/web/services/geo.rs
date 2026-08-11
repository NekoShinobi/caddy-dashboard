use actix_web::{HttpResponse, get, web};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct Query {
    since: Option<f64>,
    mode: Option<String>,
}

#[derive(Serialize)]
struct PrecisePoint {
    lat: f64,
    lng: f64,
    count: u64,
    top_ips: Vec<String>,
}

#[derive(Serialize)]
struct CountryCount {
    country: String,
    count: usize,
    top_ips: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum GeoResponse {
    Country { data: Vec<CountryCount> },
    Precise { points: Vec<PrecisePoint> },
}

fn top_ips(ip_counts: &HashMap<String, usize>, n: usize) -> Vec<String> {
    let mut v: Vec<_> = ip_counts.iter().collect();
    v.sort_unstable_by(|a, b| b.1.cmp(a.1));
    v.into_iter().take(n).map(|(ip, _)| ip.clone()).collect()
}

#[get("/geo")]
async fn get_geo(
    db: web::Data<Database>,
    geoip: web::Data<crate::geoip::GeoIpDb>,
    query: web::Query<Query>,
) -> HttpResponse {
    let mode = query.mode.as_deref().unwrap_or("country");
    if !matches!(mode, "country" | "precise") {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Invalid mode. Valid values: country, precise"}));
    }

    let all = match crate::db::load_entries(&db) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e})),
    };
    let entries: Vec<_> = match query.since {
        Some(since) => all.into_iter().filter(|e| e.ts >= since).collect(),
        None => all,
    };

    let use_precise = mode == "precise";

    if use_precise && geoip.is_some() {
        // (lat_grid, lng_grid) → (total_weight, ip → count)
        let mut grid: HashMap<(i64, i64), (f64, HashMap<String, usize>)> = HashMap::new();
        for e in &entries {
            let ip_str = if e.request.client_ip.is_empty() {
                &e.request.remote_ip
            } else {
                &e.request.client_ip
            };
            let Ok(ip) = ip_str.parse::<std::net::IpAddr>() else {
                continue;
            };
            let Some((lat, lng)) = geoip.lookup_city(ip) else {
                continue;
            };
            let key = ((lat * 100.0).round() as i64, (lng * 100.0).round() as i64);
            let entry = grid.entry(key).or_insert((0.0, HashMap::new()));
            entry.0 += 1.0;
            *entry.1.entry(ip_str.to_string()).or_insert(0) += 1;
        }
        let points: Vec<PrecisePoint> = grid
            .into_iter()
            .map(|((lat, lng), (w, ips))| PrecisePoint {
                lat: lat as f64 / 100.0,
                lng: lng as f64 / 100.0,
                count: w as u64,
                top_ips: top_ips(&ips, 10),
            })
            .collect();
        return HttpResponse::Ok().json(GeoResponse::Precise { points });
    }

    // Country mode (default or MMDB unavailable)
    // country → (count, ip → count)
    let mut counts: HashMap<String, (usize, HashMap<String, usize>)> = HashMap::new();
    for e in &entries {
        if let Some(codes) = e.request.headers.get("Cf-Ipcountry") {
            if let Some(code) = codes.first() {
                let ip_str = if e.request.client_ip.is_empty() {
                    e.request.remote_ip.clone()
                } else {
                    e.request.client_ip.clone()
                };
                let entry = counts.entry(code.clone()).or_insert((0, HashMap::new()));
                entry.0 += 1;
                *entry.1.entry(ip_str).or_insert(0) += 1;
            }
        }
    }
    let mut data: Vec<CountryCount> = counts
        .into_iter()
        .map(|(country, (count, ips))| CountryCount {
            country,
            count,
            top_ips: top_ips(&ips, 10),
        })
        .collect();
    data.sort_unstable_by(|a, b| b.count.cmp(&a.count));
    HttpResponse::Ok().json(GeoResponse::Country { data })
}
