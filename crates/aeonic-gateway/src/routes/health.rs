use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let registry = state.router.registry();
    let provider_ids = registry.provider_ids();
    let model_count = registry.all_models().len();
    let telemetry = state.telemetry.cost_summary();
    let provider_stats = state.telemetry.provider_stats();

    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "providers": provider_ids,
        "models_available": model_count,
        "telemetry": {
            "total_requests": telemetry.total_requests,
            "total_tokens": telemetry.total_tokens,
            "total_cost_usd": telemetry.total_cost_usd,
            "total_errors": telemetry.total_errors,
        },
        "provider_stats": provider_stats,
    }))
}
