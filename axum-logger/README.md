# axum-logger

POC: Axum web server with structured logging pipeline — logs flow from the app to NATS, then Vector picks them up and inserts into ClickHouse.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Rust App (axum)                                            │
│                                                             │
│  tracing event                                              │
│    ├─ stdout layer    → plain text + ANSI color             │
│    ├─ file layer      → JSON  → logs/app.YYYY-MM-DD-HH.log  │
│    └─ NATS layer      → JSON  → NATS subject "logs"         │
└──────────────────────────────┬──────────────────────────────┘
                               │
                          NATS :4222
                               │
                         ┌─────▼──────┐
                         │   Vector   │  parse JSON (VRL)
                         └─────┬──────┘
                               │
                     ┌─────────▼──────────┐
                     │    ClickHouse       │
                     │  logs.app_logs      │
                     └────────────────────┘
```

## Prerequisites

- Rust 1.95+
- Docker & Docker Compose

## Quick Start

### 1. Start infrastructure

```bash
docker compose up -d
```

Services started:
| Service | Port | URL |
|---------|------|-----|
| NATS | 4222 | — |
| NATS monitoring | 8222 | http://localhost:8222 |
| ClickHouse HTTP | 8123 | http://localhost:8123/play |
| ClickHouse native | 9000 | — |

### 2. Configure environment

```bash
cp .env.example .env  # or edit .env directly
```

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `axum_logger=debug,tower_http=debug` | Log filter |
| `NATS_URL` | `nats://localhost:4222` | NATS server URL |
| `NATS_LOG_SUBJECT` | `logs` | NATS subject to publish logs |

### 3. Run the app

```bash
cargo run
```

Server starts on `http://localhost:3000`

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |

```bash
curl http://localhost:3000/health
# {"status":"ok"}
```

## Project Structure

```
axum-logger/
├── src/
│   ├── main.rs
│   ├── routes/
│   │   └── health.rs
│   └── utils/
│       └── logger.rs       # NatsWriter + tracing layers
├── docker/
│   ├── clickhouse/
│   │   └── init.sql        # CREATE DATABASE + TABLE
│   └── vector/
│       └── vector.yaml     # NATS source → parse → ClickHouse sink
├── docker-compose.yml
├── .env
└── logs/                   # local file logs (auto-created)
```

## Logging Pipeline

### Rust side (`src/utils/logger.rs`)

Three independent tracing layers are registered:

```
stdout  → plain text (human-readable in terminal)
file    → JSON, rolling hourly → logs/app.YYYY-MM-DD-HH.log
NATS    → JSON, non-blocking via mpsc channel → background task publishes to NATS
```

`NatsWriter` implements `std::io::Write` by sending bytes into an `UnboundedSender`. A background `tokio::spawn` task reads from the channel and calls `client.publish()` — so the hot logging path never blocks.

> **Important**: `Logger::initial()` returns a `WorkerGuard` that must be held alive in `main()` for the duration of the process. Dropping it early stops the file-writer background thread.

### Log format (JSON)

Each log line published to NATS looks like:

```json
{
  "timestamp": "2026-05-11T10:00:00.000000Z",
  "level": "INFO",
  "fields": { "message": "Health check endpoint called", "tracing": "12345" },
  "target": "axum_logger::routes::health",
  "span": { "method": "GET", "uri": "/health" }
}
```

### Vector (`docker/vector/vector.yaml`)

Vector subscribes to the NATS subject using a queue group (`vector`) so multiple instances share the load safely. A VRL transform extracts key fields before inserting into ClickHouse.

### ClickHouse schema (`docker/clickhouse/init.sql`)

| Column | Type | Notes |
|--------|------|-------|
| `timestamp` | String | ISO 8601 from tracing |
| `level` | LowCardinality(String) | DEBUG / INFO / WARN / ERROR |
| `message` | String | extracted from `fields.message` |
| `target` | String | Rust module path |
| `raw_json` | String | full original JSON |
| `inserted_at` | DateTime | set by ClickHouse on insert |

Data is partitioned by day and TTL'd after **30 days**.

## Querying Logs

Open http://localhost:8123/play or use `curl`:

```sql
-- latest 20 logs
SELECT timestamp, level, message, target
FROM logs.app_logs
ORDER BY inserted_at DESC
LIMIT 20;

-- errors only
SELECT timestamp, message, raw_json
FROM logs.app_logs
WHERE level = 'ERROR'
ORDER BY inserted_at DESC;

-- logs per level (last hour)
SELECT level, count() AS cnt
FROM logs.app_logs
WHERE inserted_at >= now() - INTERVAL 1 HOUR
GROUP BY level
ORDER BY cnt DESC;
```

```bash
# via curl
curl "http://localhost:8123/" \
  --data-urlencode "query=SELECT timestamp, level, message FROM logs.app_logs ORDER BY inserted_at DESC LIMIT 10 FORMAT Pretty"
```

## Running Tests

```bash
cargo test
```

## Stopping Infrastructure

```bash
docker compose down          # keep volumes (ClickHouse data persists)
docker compose down -v       # remove volumes (fresh start)
```
