use std::io;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

// Sends each serialized log line into an mpsc channel.
// The background task reads from the channel and publishes to NATS.
struct NatsWriter {
    sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

impl io::Write for NatsWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = self.sender.send(buf.to_vec());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct NatsWriterFactory {
    sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

impl<'a> fmt::MakeWriter<'a> for NatsWriterFactory {
    type Writer = NatsWriter;

    fn make_writer(&'a self) -> Self::Writer {
        NatsWriter {
            sender: self.sender.clone(),
        }
    }
}

pub struct Logger;

impl Logger {
    pub async fn initial() -> anyhow::Result<WorkerGuard> {
        dotenv::dotenv().ok();

        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
        let nats_subject =
            std::env::var("NATS_LOG_SUBJECT").unwrap_or_else(|_| "logs".into());

        let client = async_nats::connect(&nats_url).await?;

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        tokio::spawn(async move {
            while let Some(payload) = receiver.recv().await {
                let _ = client.publish(nats_subject.clone(), payload.into()).await;
            }
        });

        let stdout_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "axum_logger=debug,tower_http=debug".into());

        let file_appender = tracing_appender::rolling::hourly("logs", "app.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "axum_logger=debug,tower_http=debug".into());

        let nats_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "axum_logger=debug,tower_http=debug".into());

        tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .with_ansi(true)
                    .with_filter(stdout_filter),
            )
            .with(
                fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(non_blocking)
                    .with_filter(file_filter),
            )
            .with(
                fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(NatsWriterFactory { sender })
                    .with_filter(nats_filter),
            )
            .init();

        Ok(guard)
    }
}
