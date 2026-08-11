set dotenv-load := true
set shell := ["bash", "-euo", "pipefail", "-c"]

# Host identity, so files the dev containers write into the bind mount stay
# owned by you instead of root. Only `up-dev` needs these.
export DEV_UID := env_var_or_default("DEV_UID", `id -u`)
export DEV_GID := env_var_or_default("DEV_GID", `id -g`)

# Supply-chain cooldown: never adopt a release younger than this. Most malicious
# package releases are found and yanked within a few days, so waiting costs
# nothing and skips the window where you would be the one to find it.
# renovate.json's `minimumReleaseAge` MUST carry the same number — Renovate
# opens the automated PRs and cannot read this file.
DEPS_MIN_AGE_DAYS := "3"

# List available recipes.
[private]
default:
    @just --list

# ── Development ───────────────────────────────────────────────────────────────

# Install Rust and frontend dependencies exactly as locked.
[group('dev')]
setup:
    cargo fetch --locked
    cd ui && bun install --frozen-lockfile

# Run the backend and frontend development servers together.
[group('dev')]
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    # kill 0 signals the whole process group, so Vite's and bacon's own
    # children die too. `kill $(jobs -p)` leaves grandchildren behind.
    trap 'kill 0' EXIT
    just dev-api &
    just dev-ui &
    wait

# Run the backend with live reload.
[group('dev')]
dev-api:
    LOG_PATH="${LOG_PATH:-access.log}" bacon --headless run

# Run the frontend development server.
[group('dev')]
dev-ui:
    cd ui && bun run dev

# ── Build ─────────────────────────────────────────────────────────────────────

# Build every release artifact.
[group('build')]
build: build-ui build-api

# Build the optimized backend binary.
[group('build')]
build-api:
    cargo build --release

# Build the production frontend.
[group('build')]
build-ui:
    cd ui && bun run build

# ── Quality ───────────────────────────────────────────────────────────────────

# Fast type-check — no formatting, no linting, no tests.
[group('checks')]
check: check-api check-ui

# Type-check the Rust workspace.
[group('checks')]
check-api:
    cargo check --workspace

# Type-check the Svelte frontend.
[group('checks')]
check-ui:
    cd ui && bun run check

# Run the Rust test suite.
[group('checks')]
test:
    cargo test --workspace

# Run all backend and frontend linters.
[group('checks')]
lint: lint-api lint-ui

# Run Clippy across the Rust workspace.
[group('checks')]
lint-api:
    cargo clippy --workspace --all-targets

# Check frontend formatting and lint rules.
[group('checks')]
lint-ui:
    cd ui && bun run lint

# Format Rust and frontend sources.
[group('checks')]
fmt:
    cargo fmt --all
    cd ui && bun run format

# Verify formatting without changing files.
[group('checks')]
fmt-check:
    cargo fmt --all -- --check
    cd ui && bunx prettier --check .

# Everything that should pass before a commit.
[group('checks')]
ci: fmt-check check lint test

# ── Docker ────────────────────────────────────────────────────────────────────

# Build a local container image.
[group('docker')]
docker-build tag="caddy-dashboard:local":
    docker build --tag "{{ tag }}" .

# Start the Compose stack in the foreground (copy compose.example.yml first).
[group('docker')]
up:
    docker compose -f compose.yml up --build

# Start the Compose stack in the background.
[group('docker')]
up-detach:
    docker compose -f compose.yml up --build -d

# Start the full containerized dev stack (parity path — `just dev` is faster).
[group('docker')]
up-dev:
    docker compose -f compose.dev.yml up --build

# Stop every stack; neither failing should prevent the other from stopping.
[group('docker')]
down:
    -docker compose -f compose.dev.yml down
    -docker compose -f compose.yml down

# Follow container logs.
[group('docker')]
logs:
    docker compose -f compose.yml logs -f

# Follow dev container logs.
[group('docker')]
logs-dev:
    docker compose -f compose.dev.yml logs -f

# Validate the Compose files without starting anything.
[group('docker')]
compose-check:
    # compose.yml is gitignored (copied from compose.example.yml), so it is
    # only validated when it is actually present.
    docker compose -f compose.dev.yml config --quiet
    docker compose -f compose.example.yml config --quiet
    if [ -f compose.yml ]; then docker compose -f compose.yml config --quiet; else echo "compose.yml absent (copy compose.example.yml) — skipped"; fi

# ── Dependencies ──────────────────────────────────────────────────────────────

# Show available dependency updates without changing anything.
[group('deps')]
deps-outdated:
    cargo update --dry-run
    cd ui && bun outdated

# Refresh lockfiles, ignoring releases younger than the cooldown.
[group('deps')]
deps-update:
    #!/usr/bin/env bash
    set -euo pipefail
    days="{{ DEPS_MIN_AGE_DAYS }}"
    seconds="$(( days * 24 * 60 * 60 ))"

    # -Zmin-publish-age is nightly-only. rust-toolchain.toml pins nightly, so
    # this should hold; failing loudly beats silently updating with no cooldown.
    if ! cargo -Z help 2>&1 | grep -q 'min-publish-age'; then
      echo "error: this cargo has no -Zmin-publish-age, so the ${days}-day cooldown cannot be enforced." >&2
      echo "Use a nightly cargo that lists min-publish-age in 'cargo -Z help'." >&2
      exit 1
    fi

    cargo update -Z min-publish-age --config "registry.global-min-publish-age = \"${days} days\""
    cd ui && bun update --latest --minimum-release-age "$seconds"

# Scan dependencies for known vulnerabilities.
[group('deps')]
deps-audit:
    cargo audit
    cd ui && bun audit

# Validate the Renovate policy that opens the automated update PRs.
[group('deps')]
deps-validate:
    bunx --package renovate renovate-config-validator --strict

# ── Sample data ───────────────────────────────────────────────────────────────

# Append one generated log entry.
[group('misc')]
inject-log:
    python3 inject-logs.py

# Continuously append generated log entries at the given interval in milliseconds.
[group('misc')]
inject-logs interval="1000":
    python3 inject-logs.py --loop --interval "{{ interval }}"

# ── Transitional aliases (remove once the new names are habit) ────────────────

alias dev-backend := dev-api
alias dev-frontend := dev-ui
alias build-backend := build-api
alias build-release := build
alias check-backend := check-api
alias lint-backend := lint-api
alias format := fmt
alias format-check := fmt-check
alias compose-up := up-detach
alias compose-down := down
