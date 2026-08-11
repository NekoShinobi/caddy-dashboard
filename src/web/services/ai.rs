use actix_web::{HttpRequest, HttpResponse, get, web};
use futures_util::{StreamExt, stream};
use redb::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
struct Query {
    since: Option<f64>,
}

pub const SETTING_KEY: &str = "ai_prompt_template";

pub const DEFAULT_PROMPT_TEMPLATE: &str = "\
You are a security-aware web traffic analyst. Analyze the following 24-hour \
Caddy access log summary. Provide:
1. A concise assessment (3-5 bullet points) flagging anything suspicious or \
anomalous. If traffic looks normal, say so briefly.
2. A short **Action Items** section listing concrete steps the operator should \
consider based on what you found.

Use markdown formatting.

{summary}

Provide your analysis and action items:";

fn build_prompt(db: &Database, since: Option<f64>) -> Result<String, &'static str> {
    let all = crate::db::load_entries(db).map_err(|_| "Database error")?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let cutoff = since.unwrap_or(now - 86400.0);
    let entries: Vec<_> = all.into_iter().filter(|e| e.ts >= cutoff).collect();

    if entries.is_empty() {
        return Err("No log entries found in the last 24 hours.");
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
        let path = e
            .request
            .uri
            .split('?')
            .next()
            .unwrap_or(&e.request.uri)
            .to_string();
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

    let s2xx: usize = status_counts
        .iter()
        .filter(|(k, _)| **k < 300)
        .map(|(_, v)| v)
        .sum();
    let s3xx: usize = status_counts
        .iter()
        .filter(|(k, _)| (300..400).contains(*k))
        .map(|(_, v)| v)
        .sum();
    let s4xx: usize = status_counts
        .iter()
        .filter(|(k, _)| (400..500).contains(*k))
        .map(|(_, v)| v)
        .sum();
    let s5xx: usize = status_counts
        .iter()
        .filter(|(k, _)| **k >= 500)
        .map(|(_, v)| v)
        .sum();

    // top 10 paths
    let mut paths: Vec<_> = path_counts.iter().collect();
    paths.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_paths: Vec<String> = paths
        .iter()
        .take(10)
        .map(|(p, c)| format!("  {} ({})", p, c))
        .collect();

    // top 10 IPs
    let mut ips: Vec<_> = ip_counts.iter().collect();
    ips.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_ips: Vec<String> = ips
        .iter()
        .take(10)
        .map(|(ip, c)| {
            let errs = ip_errors.get(*ip).copied().unwrap_or(0);
            let rate = errs as f64 / **c as f64 * 100.0;
            if errs > 0 {
                format!(
                    "  {} ({} reqs, {} errors, {:.0}% error rate)",
                    ip, c, errs, rate
                )
            } else {
                format!("  {} ({} reqs)", ip, c)
            }
        })
        .collect();

    // top error paths
    let mut epaths: Vec<_> = error_paths.iter().collect();
    epaths.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_error_paths: Vec<String> = epaths
        .iter()
        .take(10)
        .map(|(p, c)| format!("  {} ({})", p, c))
        .collect();

    // hourly distribution (24 buckets, grouped as 4-hour blocks for brevity)
    let hourly: Vec<String> = (0..6)
        .map(|block| {
            let sum: usize = hour_counts[block * 4..(block * 4 + 4)].iter().sum();
            format!(
                "  hours {:02}-{:02}: {} reqs",
                block * 4,
                block * 4 + 3,
                sum
            )
        })
        .collect();

    // top user agents (top 5)
    let mut uas: Vec<_> = ua_counts.iter().collect();
    uas.sort_unstable_by(|a, b| b.1.cmp(a.1));
    let top_uas: Vec<String> = uas
        .iter()
        .take(5)
        .map(|(ua, c)| format!("  {} ... ({})", ua, c))
        .collect();

    let summary = format!(
        "=== 24-HOUR TRAFFIC SUMMARY ===\n\
Total requests: {total}\n\
Unique IPs: {unique_ips}\n\
\n\
Status breakdown:\n\
  2xx (success):    {s2xx}\n\
  3xx (redirect):   {s3xx}\n\
  4xx (client err): {s4xx}\n\
  5xx (server err): {s5xx}\n\
\n\
Requests by hour (oldest → newest):\n\
{hourly}\n\
\n\
Top paths by volume:\n\
{top_paths}\n\
\n\
Top IPs by request count (with error rates):\n\
{top_ips}\n\
\n\
Top error paths (4xx/5xx):\n\
{top_error_paths}\n\
\n\
Top user agents:\n\
{top_uas}\n\
=== END SUMMARY ===",
        total = total,
        unique_ips = ip_counts.len(),
        hourly = hourly.join("\n"),
        top_paths = top_paths.join("\n"),
        top_ips = top_ips.join("\n"),
        top_error_paths = if top_error_paths.is_empty() {
            "  (none)".into()
        } else {
            top_error_paths.join("\n")
        },
        top_uas = top_uas.join("\n"),
    );

    let template = crate::db::get_setting(db, SETTING_KEY)
        .unwrap_or_else(|| DEFAULT_PROMPT_TEMPLATE.to_string());

    Ok(template.replace("{summary}", &summary))
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
pub async fn get_ai_analysis(
    req: HttpRequest,
    db: web::Data<Database>,
    query: web::Query<Query>,
) -> HttpResponse {
    let username = match crate::session::get_username(&req, &db) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
        }
    };
    let user = match crate::db::get_user(&db, &username) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
        }
    };
    if !user.is_admin {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Admin required"}));
    }

    let since = query.since;
    let prompt = match web::block(move || build_prompt(&db, since)).await {
        Ok(Ok(p)) => p,
        Ok(Err(msg)) => {
            return HttpResponse::UnprocessableEntity().json(serde_json::json!({"error": msg}));
        }
        Err(e) => {
            log::error!("get_ai_analysis: build_prompt panicked: {e}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal error"}));
        }
    };

    let host = crate::env::OLLAMA_HOST.trim_end_matches('/').to_string();
    let model = crate::env::OLLAMA_MODEL.clone();

    let client = reqwest::Client::new();
    let body = OllamaChatRequest {
        model,
        messages: vec![OllamaRequestMessage {
            role: "user".into(),
            content: prompt,
        }],
        stream: true,
    };

    let res = client
        .post(format!("{host}/api/chat"))
        .json(&body)
        .send()
        .await;

    let resp = match res {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status();
            log::error!("get_ai_analysis: Ollama returned HTTP {status}");
            return HttpResponse::BadGateway()
                .json(serde_json::json!({"error": format!("Ollama error: HTTP {status}")}));
        }
        Err(e) => {
            log::error!("get_ai_analysis: cannot reach Ollama at {host}: {e}");
            return HttpResponse::BadGateway()
                .json(serde_json::json!({"error": format!("Cannot reach Ollama at {host}: {e}")}));
        }
    };

    let byte_stream = resp.bytes_stream().map(|chunk| {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => {
                log::error!("get_ai_analysis: stream read error: {e}");
                return Ok(web::Bytes::from("data: {\"done\":true}\n\n"));
            }
        };
        let line = match std::str::from_utf8(&bytes) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                log::error!("get_ai_analysis: non-UTF8 chunk from Ollama: {e}");
                return Ok(web::Bytes::new());
            }
        };
        if line.is_empty() {
            return Ok(web::Bytes::new());
        }
        let sse = match serde_json::from_str::<OllamaChunk>(&line) {
            Ok(chunk) if chunk.done => "data: {\"done\":true}\n\n".to_string(),
            Ok(chunk) => {
                if let Some(err) = chunk.error {
                    log::error!("get_ai_analysis: Ollama reported error: {err}");
                    format!(
                        "data: {{\"error\":{}}}\n\n",
                        serde_json::to_string(&err).unwrap_or_default()
                    )
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
            Err(e) => {
                log::error!("get_ai_analysis: failed to parse Ollama chunk '{line}': {e}");
                String::new()
            }
        };
        Ok::<_, actix_web::Error>(web::Bytes::from(sse))
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(
            stream::once(async {
                Ok::<_, actix_web::Error>(web::Bytes::from("data: {\"started\":true}\n\n"))
            })
            .chain(byte_stream),
        )
}
