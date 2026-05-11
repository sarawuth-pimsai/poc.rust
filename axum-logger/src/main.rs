use std::net::SocketAddr;

use anyhow::Ok;
use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::utils::logger::Logger;

mod routes;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = Logger::initial().await?;
    let app = Router::new()
        .merge(routes::health::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
