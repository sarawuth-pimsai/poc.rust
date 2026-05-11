CREATE DATABASE IF NOT EXISTS logs;

CREATE TABLE IF NOT EXISTS logs.app_logs
(
    timestamp   String,
    level       LowCardinality(String),
    message     String,
    target      String,
    raw_json    String,
    inserted_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
PARTITION BY toYYYYMMDD(inserted_at)
ORDER BY (inserted_at, level)
TTL inserted_at + INTERVAL 30 DAY;
