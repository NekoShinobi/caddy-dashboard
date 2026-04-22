FROM ghcr.io/nekoshinobi/rust-chef-sccache:main AS base
FROM base AS planner
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef prepare --recipe-path recipe.json

FROM oven/bun:1-alpine AS frontend-builder
WORKDIR /app/ui
COPY ui/package.json ui/bun.lock* ./
RUN bun install --frozen-lockfile
COPY ui/ .
RUN bun run build

FROM base AS builder
WORKDIR /app

COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release

FROM ubuntu:24.04
WORKDIR /app

RUN DEBIAN_FRONTEND=noninteractive apt update && apt upgrade -y && apt clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/caddy-dashboard caddy-dashboard
COPY --from=frontend-builder /app/ui/build /app/static

CMD ["/app/caddy-dashboard"]
