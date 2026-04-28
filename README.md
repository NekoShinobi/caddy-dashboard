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

## Authentication

On first start, navigate to the dashboard and create the initial admin account. Subsequent registrations are disabled — additional users are managed through the admin panel (Settings → User Management).

To reset the user database (e.g., locked out): set `USER_DATABASE_RESET=true` in the environment, restart the container, then remove the variable and create a new admin account. All sessions are also invalidated.

### OIDC / SSO

Set `OIDC_CLIENT_ID` to enable SSO login alongside (or instead of) local accounts.

**Provider setup:** register the following redirect URI with your identity provider:
```
{BASE_URL}/api/auth/oidc/callback
```

**Scopes:** the default `openid email profile` is sufficient for most providers. An email address is required — logins without one are rejected. If `email_verified` is present and `false` the login is also rejected.

**User matching:** OIDC users are matched to existing accounts by email (case-insensitive). A new account is created on first login if no match exists. The username is derived from `preferred_username`, the local part of the email, or the OIDC `sub` claim.

**Admin rights:**
- If `OIDC_ADMIN_CLAIM` + `OIDC_ADMIN_VALUE` are set, admin status is synced from the claim on every login.
- If neither is set, the first OIDC user to log in gets admin rights; subsequent users are non-admin by default.

**Logout:** if the provider exposes an `end_session_endpoint` in its discovery document, logging out from the dashboard also triggers RP-initiated logout at the provider (with `id_token_hint`).

**OIDC-only mode:** set `OIDC_DISABLE_LOGIN=true` to hide the local login form and block the login/signup API endpoints entirely.

## Configuration

All configuration via environment variables:

### Core

| Variable              | Default                  | Description                                                       |
|-----------------------|--------------------------|-------------------------------------------------------------------|
| `LOG_PATH`            | `/config/access.log`     | Path to Caddy access log file                                     |
| `DATA_DIR`            | `./data`                 | Directory for the redb database                                   |
| `PORT`                | `9080`                   | HTTP port                                                         |
| `GEOIP_DB`            | *(embedded DB-IP Lite)*  | Path to an external MaxMind-compatible `.mmdb` file               |
| `RETENTION_DAYS`      | `0` (disabled)           | Purge entries older than N days (0 = keep forever)                |
| `OLLAMA_HOST`         | `http://localhost:11434` | Ollama API base URL for AI analysis                               |
| `OLLAMA_MODEL`        | `llama3.2`               | Ollama model name (must be pulled)                                |
| `COOKIE_SECURE`       | `true`                   | Set `false` only for local dev over plain HTTP                    |
| `BASE_URL`            | *(derived from request)* | Public base URL, e.g. `https://dash.example.com` — required for OIDC behind a reverse proxy |
| `USER_DATABASE_RESET` | `false`                  | Set `true` to wipe all users and sessions on next startup         |

In Docker, `LOG_PATH` defaults to `/config/access.log` and `DATA_DIR` defaults to `/data`.

### OIDC

| Variable               | Default                   | Description                                                                   |
|------------------------|---------------------------|-------------------------------------------------------------------------------|
| `OIDC_CLIENT_ID`       | *(unset)*                 | Client ID — setting this enables OIDC                                         |
| `OIDC_CLIENT_SECRET`   | *(unset)*                 | Client secret                                                                 |
| `OIDC_ISSUER_URL`      | *(unset)*                 | Issuer base URL (discovery doc fetched from `{issuer}/.well-known/openid-configuration`) |
| `OIDC_SCOPES`          | `openid email profile`    | Space-separated scopes to request                                             |
| `OIDC_ADMIN_CLAIM`     | *(unset)*                 | Claim name to check for admin rights (e.g. `groups`, `roles`)                 |
| `OIDC_ADMIN_VALUE`     | *(unset)*                 | Value within `OIDC_ADMIN_CLAIM` that grants admin (e.g. `admins`)             |
| `OIDC_PROVIDERS_NAME`  | `SSO`                     | Label shown on the login button                                               |
| `OIDC_PROVIDER_LOGO_URL` | *(unset)*               | Optional logo URL shown on the login button                                   |
| `OIDC_DISABLE_LOGIN`   | `false`                   | Hide local login form and block login/signup endpoints                        |

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
│   ├── db.rs              redb setup and helpers (logs, users, sessions, settings, oidc_tokens)
│   ├── auth.rs            Password hashing (Argon2id)
│   ├── session.rs         Session token generation and cookie helpers
│   ├── oidc.rs            OIDC discovery, token exchange, userinfo, admin claim check
│   ├── login_throttle.rs  Progressive delay on repeated login failures
│   ├── geoip.rs           GeoIP lookup (embedded DB-IP Lite or external file)
│   ├── ingest.rs          Background log ingestion task
│   ├── log_parser.rs      Caddy JSON log structs
│   └── web/
│       ├── mod.rs         actix-web server + SPA fallback
│       ├── middleware.rs  RequireAuth middleware
│       └── services/      API route handlers
│           ├── auth.rs    Login, logout, signup, password change
│           ├── oidc.rs    OIDC login initiation and callback
│           ├── admin.rs   User management (admin only)
│           ├── settings.rs Site settings (AI prompt)
│           ├── logs.rs    Log query, filtering, CSV export
│           ├── reports.rs Error rate and large payload reports
│           └── ai.rs      Ollama AI analysis (SSE streaming)
├── ui/                    SvelteKit frontend
│   └── src/
│       ├── lib/
│       │   ├── auth.svelte.ts       Auth state, OIDC config, login/logout helpers
│       │   ├── crypto.ts            Client-side SHA-256 password hashing
│       │   └── components/
│       │       └── AuthGate.svelte  Login/SSO gate wrapping authenticated routes
│       └── routes/
│           ├── +page.svelte   Overview dashboard
│           ├── logs/          Log table with filters, modal, CSV export
│           ├── graphs/        Time-series charts
│           ├── map/           Geographic origin map (cluster mode)
│           ├── reports/       Reports + AI analysis
│           └── settings/      Account, user management, site settings
├── inject-logs.py         Test data utility
├── Dockerfile
└── compose.example.yml
```
