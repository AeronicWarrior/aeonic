use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let registry = state.router.registry();
    let provider_ids = registry.provider_ids();
    let model_count = registry.all_models().len();

    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "providers": provider_ids,
        "models_available": model_count,
    }))
}
