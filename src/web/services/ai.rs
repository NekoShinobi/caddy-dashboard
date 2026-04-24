use actix_web::{get, web, HttpResponse};
use futures_util::{stream, StreamExt};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn build_prompt(db: &Database) -> String {
    let all = crate::db::load_entries(db);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let cutoff = now - 86400.0;
    let entries: Vec<_> = all.into_iter().filter(|e| e.ts >= cutoff).collect();

    if entries.is_empty() {
        return "No log entries found in the last 24 hours.".into();
    }

    let total = entries.len();
    let mut status_counts: HashMap<u16, usize> = HashMap::new();
    let mut path_counts: HashMap<String, usize> = HashMap::new();
    let mut ip_counts: HashMap<String, usize> = HashMap::new();
    let mut ip_errors: HashMap<String, usize> = HashMap::new();
    let mut hour_counts = [0usize; 24];
    let mut ua_counts: HashMap<String, usize> = HashMap::new();
    let mut error_paths: HashMap<String, usize> = HashMap::new();

    for e in &entries {
        *status_counts.entry(e.status).or_insert(0) += 1;

        // strip query string for path aggregation
        let path = e.request.uri.split('?').next().unwrap_or(&e.request.uri).to_string();
        *path_counts.entry(path.clone()).or_insert(0) += 1;

        let ip = if e.request.client_ip.is_empty() {
            e.request.remote_ip.clone()
        } else {
            e.request.client_ip.clone()
        };
        *ip_counts.entry(ip.clone()).or_insert(0) += 1;

        if e.status >= 400 {
            *ip_errors.entry(ip).or_insert(0) += 1;
            *error_paths.entry(path).or_insert(0) += 1;
        }

        let hour = (((e.ts - cutoff) / 3600.0) as usize).min(23);
        hour_counts[hour] += 1;

        if let Some(ua_vals) = e.request.headers.get("User-Agent") {
            if let Some(ua) = ua_vals.first() {
                // coarse bucket: first 60 chars
                let key = ua.chars().take(60).collect::<String>();
                *ua_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    let s2xx: usize = status_counts.iter().filter(|(k, _)| **k < 300).map(|(_, v)| v).sum();
    let s3xx: usize = status_counts.iter().filter(|(k, _)| (300..400).contains(*k)).map(|(_, v)| v).sum();
    let s4xx: usize = status_counts.iter().filter(|(k, _)| (400..500).contains(*k)).map(|(_, v)| v).sum();
    let s5xx: usize = status_counts.iter().filter(|(k, _)| **k >= 500).map(|(_, v)| v).sum();

    // top 10 paths
    let mut paths: Vec<_> = path_counts.iter().collect();
    paths.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_paths: Vec<String> = paths.iter().take(10)
        .map(|(p, c)| format!("  {} ({})", p, c))
        .collect();

    // top 10 IPs
    let mut ips: Vec<_> = ip_counts.iter().collect();
    ips.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_ips: Vec<String> = ips.iter().take(10).map(|(ip, c)| {
        let errs = ip_errors.get(*ip).copied().unwrap_or(0);
        let rate = errs as f64 / **c as f64 * 100.0;
        if errs > 0 {
            format!("  {} ({} reqs, {} errors, {:.0}% error rate)", ip, c, errs, rate)
        } else {
            format!("  {} ({} reqs)", ip, c)
        }
    }).collect();

    // top error paths
    let mut epaths: Vec<_> = error_paths.iter().collect();
    epaths.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_error_paths: Vec<String> = epaths.iter().take(10)
        .map(|(p, c)| format!("  {} ({})", p, c))
        .collect();

    // hourly distribution (24 buckets, grouped as 4-hour blocks for brevity)
    let hourly: Vec<String> = (0..6).map(|block| {
        let sum: usize = hour_counts[block*4..(block*4+4)].iter().sum();
        format!("  hours {:02}-{:02}: {} reqs", block*4, block*4+3, sum)
    }).collect();

    // top user agents (top 5)
    let mut uas: Vec<_> = ua_counts.iter().collect();
    uas.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_uas: Vec<String> = uas.iter().take(5)
        .map(|(ua, c)| format!("  {} ... ({})", ua, c))
        .collect();

    format!(
        r#"You are a security-aware web traffic analyst. Analyze the following 24-hour Caddy access log summary. Provide:
1. A concise assessment (3-5 bullet points) flagging anything suspicious or anomalous. If traffic looks normal, say so briefly.
2. A short **Action Items** section listing concrete steps the operator should consider based on what you found.

Use markdown formatting.

=== 24-HOUR TRAFFIC SUMMARY ===
Total requests: {total}
Unique IPs: {unique_ips}

Status breakdown:
  2xx (success):   {s2xx}
  3xx (redirect):  {s3xx}
  4xx (client err): {s4xx}
  5xx (server err): {s5xx}

Requests by hour (oldest → newest):
{hourly}

Top paths by volume:
{top_paths}

Top IPs by request count (with error rates):
{top_ips}

Top error paths (4xx/5xx):
{top_error_paths}

Top user agents:
{top_uas}
=== END SUMMARY ===

Provide your analysis and action items:"#,
        total = total,
        unique_ips = ip_counts.len(),
        hourly = hourly.join("\n"),
        top_paths = top_paths.join("\n"),
        top_ips = top_ips.join("\n"),
        top_error_paths = if top_error_paths.is_empty() { "  (none)".into() } else { top_error_paths.join("\n") },
        top_uas = top_uas.join("\n"),
    )
}

#[derive(Deserialize)]
struct OllamaChunk {
    message: Option<OllamaMessage>,
    done: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaRequestMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct OllamaRequestMessage {
    role: String,
    content: String,
}

#[get("/reports/ai-analysis")]
pub async fn get_ai_analysis(db: web::Data<Database>) -> HttpResponse {
    let prompt = web::block(move || build_prompt(&db)).await.unwrap_or_default();

    let host = crate::env::OLLAMA_HOST.trim_end_matches('/').to_string();
    let model = crate::env::OLLAMA_MODEL.clone();

    let client = reqwest::Client::new();
    let body = OllamaChatRequest {
        model,
        messages: vec![OllamaRequestMessage { role: "user".into(), content: prompt }],
        stream: true,
    };

    log::info!("Ollama request body: {}", serde_json::to_string(&body).unwrap_or_default());

    let res = client
        .post(format!("{host}/api/chat"))
        .json(&body)
        .send()
        .await;

    let resp = match res {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status();
            return HttpResponse::BadGateway().body(format!("Ollama error: HTTP {status}"));
        }
        Err(e) => {
            return HttpResponse::BadGateway()
                .body(format!("Cannot reach Ollama at {host}: {e}"));
        }
    };

    let byte_stream = resp.bytes_stream().map(|chunk| {
        let bytes = match chunk {
            Ok(b) => b,
            Err(_) => return Ok(web::Bytes::from("data: {\"done\":true}\n\n")),
        };
        let line = std::str::from_utf8(&bytes).unwrap_or("").trim().to_string();
        if line.is_empty() {
            return Ok(web::Bytes::new());
        }
        let sse = match serde_json::from_str::<OllamaChunk>(&line) {
            Ok(chunk) if chunk.done => "data: {\"done\":true}\n\n".to_string(),
            Ok(chunk) => {
                if let Some(err) = chunk.error {
                    format!("data: {{\"error\":{}}}\n\n", serde_json::to_string(&err).unwrap_or_default())
                } else if let Some(msg) = chunk.message {
                    if !msg.content.is_empty() {
                        let escaped = serde_json::to_string(&msg.content).unwrap_or_default();
                        format!("data: {{\"token\":{escaped}}}\n\n")
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            Err(_) => String::new(),
        };
        Ok::<_, actix_web::Error>(web::Bytes::from(sse))
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream::once(async {
            Ok::<_, actix_web::Error>(web::Bytes::from("data: {\"started\":true}\n\n"))
        }).chain(byte_stream))
}
