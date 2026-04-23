use actix_web::{get, web, HttpResponse};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct Query {
    since: Option<f64>,
}

#[derive(Serialize)]
struct CountryCount {
    country: String,
    count: usize,
}

#[get("/geo")]
async fn get_geo(db: web::Data<Database>, query: web::Query<Query>) -> HttpResponse {
    let all = crate::db::load_entries(&db);
    let entries: Vec<_> = match query.since {
        Some(since) => all.into_iter().filter(|e| e.ts >= since).collect(),
        None => all,
    };
    let mut counts: HashMap<String, usize> = HashMap::new();

    for e in &entries {
        if let Some(codes) = e.request.headers.get("Cf-Ipcountry") {
            if let Some(code) = codes.first() {
                *counts.entry(code.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut result: Vec<CountryCount> = counts
        .into_iter()
        .map(|(country, count)| CountryCount { country, count })
        .collect();
    result.sort_unstable_by(|a, b| b.count.cmp(&a.count));

    HttpResponse::Ok().json(result)
}
