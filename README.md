# caddy-dashboard

A self-hosted analytics dashboard for [Caddy](https://caddyserver.com/) access logs.

**Stack:** Rust (actix-web) · SvelteKit · Tailwind CSS v4 · redb · Chart.js · Leaflet


## Goals

I wanted to create a project where it was super easy to deploy and immediately get some decent visualizations into what kind of traffic you have.

This is the recommended log settings that I use for my personal Caddy instance

- Excludes inter-traffic, because I want more visibility on external traffic, not internal.
- Reasonable rollover defaults, but currently caddy-dashboard will only fetch incoming logs that it sees. It does not retroactively retrieve logs.
- You can just get rid of roll over logs if you want to rely on caddy-dashboard to hold onto historical data.
- Excludes Uptime Kuma to reduce some noise.

```
(log_settings) {
    log {
        output file /config/access.log {
            roll_size 30MB
            roll_keep 5
            roll_keep_for 720h
        }
        level debug
    }
    @name {
        client_ip 172.18.0.0/12
        client_ip 192.168.0.0/12
    }

    @uptime-kuma-agent {
        header User-Agent Uptime-Kuma*
    }

    # Skip logging for specific IP ranges
    log_skip @name
    log_skip @uptime-kuma-agent
}
```


## Features

- **Overview** — total requests, status code breakdown, top hosts/IPs/paths (host + path combined), slowest paths (avg + p99 duration)
- **Logs** — paginated, filterable log table with configurable page size and direct page navigation
  - Advanced filter syntax: `host:`, `path:`, `ip:`, `status:`, `method:`, `size:>N`, `size:<N` — prefix with `-` for negation (e.g. `-status:200`)
  - Inline filter help (`?` button)
  - Click any row to open a full detail modal with all request/response fields and Copy JSON
  - CSV export (applies active filters)
- **Graphs** — requests over time, response duration (avg/median/p99), payload size (avg/median/p99), unique hosts — bucketed by minute/hour/day — with human-readable axis units (ms/s/m, B/KB/MB)
- **Map** — request origins plotted on an interactive world map using an embedded DB-IP Lite GeoIP database (auto-downloaded at build time). Falls back to Cloudflare's `Cf-Ipcountry` header for country-level placement. Individual points cluster when zoomed out.
- **Reports** — security-focused analysis views:
  - **High Error Rate by IP** — IPs with elevated 4xx/5xx rates, with per-endpoint breakdown and direct Logs link
  - **Largest Response Payloads** — top 100 entries by response size
  - **AI Traffic Analysis** — streams a 24-hour traffic summary to a local [Ollama](https://ollama.com/) instance for anomaly detection and action items (requires `OLLAMA_HOST` and `OLLAMA_MODEL`)
- **Color themes** — 6 presets (Default, Nord, Dracula, Catppuccin, Sunset, Neon) applied to charts, persisted to localStorage
- **Light/dark mode** — persisted to localStorage
- **Real-time streaming** — SSE endpoint (`/api/logs/stream`) tails new log entries as they arrive
- **Log rotation aware** — detects inode changes and file truncation, seamlessly resumes from new file
- **Tail-only ingestion** — on first start, skips existing log content and ingests only new entries going forward
- **Data retention** — optional automatic purge of entries older than N days (`RETENTION_DAYS`)
- **Anonymize mode** — toggle to blur IP addresses across the UI

## Screenshots

![Overview](screenshots/overview.png)
![Graphs](screenshots/graphs.png)
![Logs](screenshots/logs.png)
![Map](screenshots/map.png)

## Architecture

```
access.log ──► Ingestion task (250ms poll) ──► redb (./data/caddy.db)
                                           └──► broadcast channel ──► SSE clients
All API reads ──► redb
```

Log entries are parsed once on ingest and stored in an embedded [redb](https://github.com/cberner/redb) database. The source log file is treated as transient; the database is the persistent store.

On first start the ingestion task records the current end-of-file position and tails only new entries from that point forward. After a log rotation (inode change or truncation) the new file is read from the beginning.

The DB-IP Lite City MMDB is downloaded automatically during `cargo build` and embedded into the binary via `include_bytes!`. Set `SKIP_DBIP_DOWNLOAD=1` to skip the download (e.g. in CI).

## Getting Started

### Local development

```bash
# Backend (defaults: LOG_PATH=access.log, DATA_DIR=./data, PORT=9080)
LOG_PATH=access.log cargo run

# Frontend (proxies /api → :9080)
cd ui
bun install
bun run dev
```

### Docker

```bash
cp compose.example.yml compose.yml
# Edit compose.yml — set the access.log volume source
docker compose up -d
```

### Test data

`inject-logs.py` streams sample entries from `access-gen.log` into `access.log`:

```bash
python3 inject-logs.py                       # append one entry
python3 inject-logs.py --loop                # one entry per second (default)
python3 inject-logs.py --loop --interval 200 # one entry every 200ms
```

Each entry's timestamp is set to the current time.

## Configuration

All configuration via environment variables:

| Variable          | Default                  | Description                                            |
|-------------------|--------------------------|--------------------------------------------------------|
| `LOG_PATH`        | `/config/access.log`     | Path to Caddy access log file                          |
| `DATA_DIR`        | `./data`                 | Directory for the redb database                        |
| `PORT`            | `9080`                   | HTTP port                                              |
| `GEOIP_DB`        | *(embedded DB-IP Lite)*  | Path to an external MaxMind-compatible `.mmdb` file    |
| `RETENTION_DAYS`  | `0` (disabled)           | Purge entries older than N days (0 = keep forever)     |
| `OLLAMA_HOST`     | `http://localhost:11434` | Ollama API base URL for AI analysis                    |
| `OLLAMA_MODEL`    | `llama3.2`               | Ollama model name (must be pulled)                     |

In Docker, `LOG_PATH` defaults to `/config/access.log` and `DATA_DIR` defaults to `/data`.

## API

| Method | Path                           | Description                                                                 |
|--------|--------------------------------|-----------------------------------------------------------------------------|
| GET    | `/api/stats`                   | Aggregated stats (status codes, top lists, slowest paths)                   |
| GET    | `/api/logs`                    | Paginated + filtered log entries                                            |
| GET    | `/api/logs/export`             | CSV export of filtered log entries                                          |
| GET    | `/api/logs/stream`             | SSE stream of new log entries in real time                                  |
| GET    | `/api/timeline`                | Time-bucketed stats (`bucket=minute\|hour\|day`)                            |
| GET    | `/api/geo`                     | Request counts and coordinates for map rendering                            |
| GET    | `/api/reports/error-rates`     | IPs with high 4xx/5xx error rates and per-endpoint breakdown                |
| GET    | `/api/reports/large-payloads`  | Top 100 entries by response body size                                       |
| GET    | `/api/reports/ai-analysis`     | SSE stream of Ollama AI analysis of the last 24 hours of traffic            |

### Log filter parameters (`/api/logs`, `/api/logs/export`)

| Param        | Example          | Description                    |
|--------------|------------------|--------------------------------|
| `host`       | `example.com`    | Exact or glob match (`*`)      |
| `path`       | `/api/*`         | Exact or glob match            |
| `ip`         | `1.2.3.4`        | Client IP                      |
| `status`     | `4xx` or `200`   | Status code or range           |
| `method`     | `POST`           | HTTP method                    |
| `size_gt`    | `1048576`        | Response size > N bytes        |
| `size_lt`    | `1048576`        | Response size < N bytes        |
| `not_host`   | `example.com`    | Exclude host                   |
| `not_path`   | `/health`        | Exclude path                   |
| `not_ip`     | `1.2.3.4`        | Exclude IP                     |
| `not_status` | `200`            | Exclude status                 |
| `not_method` | `GET`            | Exclude method                 |

## Project Structure

```
caddy-dashboard/
├── build.rs               Downloads and embeds DB-IP Lite GeoIP MMDB at compile time
├── src/
│   ├── main.rs            Entry point
│   ├── env.rs             Environment variable config
│   ├── db.rs              redb setup and helpers
│   ├── geoip.rs           GeoIP lookup (embedded DB-IP Lite or external file)
│   ├── ingest.rs          Background log ingestion task
│   ├── log_parser.rs      Caddy JSON log structs
│   └── web/
│       ├── mod.rs         actix-web server + SPA fallback
│       └── services/      API route handlers
│           ├── logs.rs    Log query, filtering, CSV export
│           ├── reports.rs Error rate and large payload reports
│           └── ai.rs      Ollama AI analysis (SSE streaming)
├── ui/                    SvelteKit frontend
│   └── src/routes/
│       ├── +page.svelte   Overview dashboard
│       ├── logs/          Log table with filters, modal, CSV export
│       ├── graphs/        Time-series charts
│       ├── map/           Geographic origin map (cluster mode)
│       └── reports/       Reports + AI analysis
├── inject-logs.py         Test data utility
├── Dockerfile
└── compose.example.yml
```
