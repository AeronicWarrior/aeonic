use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::AppState;

/// GET /v1/models — OpenAI-compatible model listing.
pub async fn list_models(State(state): State<Arc<AppState>>) -> Json<Value> {
    let models = state.router.registry().all_models();

    let data: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "object": "model",
                "created": 1_700_000_000u64,
                "owned_by": m.provider,
                "aeonic": {
                    "provider": m.provider,
                    "display_name": m.display_name,
                    "capability": format!("{:?}", m.capability).to_lowercase(),
                    "context_window": m.context_window,
                    "cost_input_per_1m_usd": m.cost_input_per_1m,
                    "cost_output_per_1m_usd": m.cost_output_per_1m,
                    "supports_streaming": m.supports_streaming,
                    "supports_vision": m.supports_vision,
                    "supports_tools": m.supports_tools,
                }
            })
        })
        .collect();

    Json(json!({
        "object": "list",
        "data": data,
    }))
}
