# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build all workspace members
cargo build

# Run a specific workspace member
cargo run -p axum-logger
cargo run -p grpc
cargo run -p logging

# Run tests (all workspace)
cargo test

# Run tests for a specific member
cargo test -p axum-logger

# Run a single test by name
cargo test -p axum-logger <test_name>

# Check without building
cargo check
```

`axum-logger` requires infrastructure (NATS + ClickHouse + Vector) before running:

```bash
cd axum-logger
docker compose up -d      # start infrastructure
cp .env.example .env      # configure env (first time)
cargo run -p axum-logger

docker compose down       # stop (keep volumes)
docker compose down -v    # stop + wipe volumes
```

## Architecture

This is a Cargo workspace (`resolver = "3"`, edition 2024) with three independent POC crates:

### `axum-logger` — structured logging pipeline
Axum HTTP server that fans out every tracing event to three sinks simultaneously:

```
tracing event
  ├── stdout layer  → ANSI colored text (human-readable)
  ├── file layer    → JSON, hourly rolling → logs/app.YYYY-MM-DD-HH.log
  └── NATS layer    → JSON → NATS subject "logs"
                           → Vector (VRL transform) → ClickHouse logs.app_logs
```

**Key design constraint**: `Logger::initial()` (`src/utils/logger.rs`) returns a `WorkerGuard` that **must be held alive in `main()`** for the entire process lifetime. Dropping it early silently stops the file-writer background thread.

The NATS sink is non-blocking: `NatsWriter` implements `io::Write` by pushing bytes into a `tokio::sync::mpsc::UnboundedSender`. A background `tokio::spawn` task drains the channel and calls `client.publish()`, keeping the hot log path off the async executor.

Environment variables (loaded from `.env` via `dotenv`):
| Variable | Default |
|---|---|
| `RUST_LOG` | `axum_logger=debug,tower_http=debug` |
| `NATS_URL` | `nats://localhost:4222` |
| `NATS_LOG_SUBJECT` | `logs` |

Integration tests use `axum-test` crate (see `[dev-dependencies]`).

### `grpc` — gRPC with tonic/prost
Minimal gRPC POC. Proto definition lives at `grpc/proto/greeter.proto`. `build.rs` compiles it at build time via `tonic_prost_build` generating both server and client stubs. The compiled proto is included via `tonic::include_proto!("greeter")`.

### `logging` — standalone logging library
Simpler logger (`logging/src/lib.rs`) that provides daily-rolling JSON file output only (no NATS, no stdout). `Logger::new()` returns a `WorkerGuard` — same ownership contract as `axum-logger`.
