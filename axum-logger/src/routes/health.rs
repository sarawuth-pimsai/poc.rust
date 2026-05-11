use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}
async fn health_check() -> Json<HealthResponse> {
    tracing::info!(tracing = "12345", "Health check endpoint called");
    Json(HealthResponse { status: "ok" })
}

pub fn router() -> axum::Router {
    axum::Router::new().route("/health", axum::routing::get(health_check))
}
