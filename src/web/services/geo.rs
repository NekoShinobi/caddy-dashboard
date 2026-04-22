use actix_web::{get, web, HttpResponse};
use redb::Database;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct CountryCount {
    country: String,
    count: usize,
}

#[get("/geo")]
async fn get_geo(db: web::Data<Database>) -> HttpResponse {
    let entries = crate::db::load_entries(&db);
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
