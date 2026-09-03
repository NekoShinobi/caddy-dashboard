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

fn build_geo_response(
    db: &Database,
    geoip: &crate::geoip::GeoIpDb,
    since: Option<f64>,
    use_precise: bool,
) -> Result<GeoResponse, String> {
    if use_precise && geoip.is_some() {
        // (lat_grid, lng_grid) -> (total_weight, ip -> count)
        let mut grid: HashMap<(i64, i64), (f64, HashMap<String, usize>)> = HashMap::new();
        crate::db::visit_logs_since(db, since, |entry| {
            let ip_str = if entry.request.client_ip.is_empty() {
                &entry.request.remote_ip
            } else {
                &entry.request.client_ip
            };
            let Ok(ip) = ip_str.parse::<std::net::IpAddr>() else {
                return;
            };
            let Some((lat, lng)) = geoip.lookup_city(ip) else {
                return;
            };
            let key = ((lat * 100.0).round() as i64, (lng * 100.0).round() as i64);
            let point = grid.entry(key).or_insert_with(|| (0.0, HashMap::new()));
            point.0 += 1.0;
            *point.1.entry(ip_str.to_owned()).or_insert(0) += 1;
        })?;

        let points = grid
            .into_iter()
            .map(|((lat, lng), (weight, ips))| PrecisePoint {
                lat: lat as f64 / 100.0,
                lng: lng as f64 / 100.0,
                count: weight as u64,
                top_ips: top_ips(&ips, 10),
            })
            .collect();
        return Ok(GeoResponse::Precise { points });
    }

    // country -> (count, ip -> count)
    let mut counts: HashMap<String, (usize, HashMap<String, usize>)> = HashMap::new();
    crate::db::visit_logs_since(db, since, |entry| {
        let Some(code) = entry
            .request
            .headers
            .get("Cf-Ipcountry")
            .and_then(|codes| codes.first())
        else {
            return;
        };
        let ip = if entry.request.client_ip.is_empty() {
            &entry.request.remote_ip
        } else {
            &entry.request.client_ip
        };
        let country = counts
            .entry(code.clone())
            .or_insert_with(|| (0, HashMap::new()));
        country.0 += 1;
        *country.1.entry(ip.to_owned()).or_insert(0) += 1;
    })?;

    let mut data: Vec<CountryCount> = counts
        .into_iter()
        .map(|(country, (count, ips))| CountryCount {
            country,
            count,
            top_ips: top_ips(&ips, 10),
        })
        .collect();
    data.sort_unstable_by(|a, b| b.count.cmp(&a.count));
    Ok(GeoResponse::Country { data })
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

    let use_precise = mode == "precise";
    let since = query.since;
    let db = db.into_inner();
    let geoip = geoip.into_inner();
    match web::block(move || build_geo_response(&db, &geoip, since, use_precise)).await {
        Ok(Ok(response)) => HttpResponse::Ok().json(response),
        Ok(Err(error)) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": error}))
        }
        Err(error) => {
            log::error!("geo blocking task: {error}");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"}))
        }
    }
}
