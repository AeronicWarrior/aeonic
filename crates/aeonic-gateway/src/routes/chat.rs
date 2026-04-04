use crate::{error::GatewayError, state::AppState};
use aeonic_core::{
    traits::Router,
    types::{Message, MessageContent, MessageRole, Request, RoutingHints, RoutingStrategy},
};
use axum::{
    extract::State,
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

// ── OpenAI-compatible request/response wire types ─────────────────────────────

#[derive(Deserialize)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<OaiMessage>,
    #[serde(default)]
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    // Aeonic routing extensions (ignored by OpenAI clients, used by Aeonic)
    #[serde(default)]
    pub aeonic: Option<AeonicExtensions>,
}

#[derive(Deserialize)]
pub struct OaiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Default)]
pub struct AeonicExtensions {
    pub strategy: Option<String>,
    pub max_cost_per_1m: Option<f64>,
    pub max_latency_ms: Option<u64>,
    pub task_type: Option<String>,
    pub allow_providers: Option<Vec<String>>,
    pub deny_providers: Option<Vec<String>>,
}

/// POST /v1/chat/completions — OpenAI-compatible endpoint.
/// Accepts the same JSON shape as the OpenAI API, so any
/// OpenAI-compatible client works without modification.
#[instrument(skip(state, body))]
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatCompletionRequest>,
) -> Result<Response, GatewayError> {
    let request = build_aeonic_request(body)?;

    if request.params.stream {
        handle_stream(state, request).await
    } else {
        handle_complete(state, request).await
    }
}

/// POST /aeonic/v1/route — Aeonic-native endpoint.
/// Same as chat_completions but with richer routing control exposed.
pub async fn aeonic_route(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ChatCompletionRequest>,
) -> Result<Response, GatewayError> {
    chat_completions(State(state), Json(body)).await
}

async fn handle_complete(
    state: Arc<AppState>,
    request: Request,
) -> Result<Response, GatewayError> {
    let request_clone = request.clone();
    let response = state.router.route(request).await.map_err(GatewayError)?;
    state.telemetry.record(&request_clone, &response);


    let oai_response = json!({
        "id": format!("chatcmpl-{}", response.id),
        "object": "chat.completion",
        "created": response.created_at.timestamp(),
        "model": response.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": response.message.text(),
            },
            "finish_reason": format!("{:?}", response.finish_reason).to_lowercase(),
        }],
        "usage": {
            "prompt_tokens": response.usage.prompt_tokens,
            "completion_tokens": response.usage.completion_tokens,
            "total_tokens": response.usage.total_tokens,
        },
        "aeonic": {
            "provider": response.provider,
            "latency_ms": response.latency_ms,
            "cost_usd": response.usage.cost_usd,
        }
    });

    Ok(Json(oai_response).into_response())
}

async fn handle_stream(
    state: Arc<AppState>,
    request: Request,
) -> Result<Response, GatewayError> {
    let stream = state
        .router
        .route_stream(request)
        .await
        .map_err(GatewayError)?;

    let sse_stream = stream.map(|chunk_result| {
        match chunk_result {
            Ok(chunk) => {
                let data = json!({
                    "id": format!("chatcmpl-{}", chunk.request_id),
                    "object": "chat.completion.chunk",
                    "model": chunk.model,
                    "choices": [{
                        "index": 0,
                        "delta": { "content": chunk.delta },
                        "finish_reason": chunk.finish_reason
                            .map(|r| format!("{r:?}").to_lowercase()),
                    }],
                    "aeonic": { "provider": chunk.provider }
                });
                Ok::<axum::response::sse::Event, axum::Error>(axum::response::sse::Event::default()
                    .data(data.to_string()))
            }
            Err(e) => {
                let err_data = json!({ "error": e.to_string() });
                Ok::<axum::response::sse::Event, axum::Error>(axum::response::sse::Event::default()
                    .data(err_data.to_string()))
            }
        }
    });

    Ok(Sse::new(sse_stream).into_response())
}

fn build_aeonic_request(body: ChatCompletionRequest) -> Result<Request, GatewayError> {
    let messages: Vec<Message> = body
        .messages
        .into_iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system"    => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                "tool"      => MessageRole::Tool,
                _           => MessageRole::User,
            };
            Message {
                role,
                content: MessageContent::Text(m.content),
                name: None,
                tool_call_id: None,
            }
        })
        .collect();

    let ext = body.aeonic.unwrap_or_default();

    let strategy = match ext.strategy.as_deref() {
        Some("min_cost")    => RoutingStrategy::MinCost,
        Some("max_quality") => RoutingStrategy::MaxQuality,
        Some("min_latency") => RoutingStrategy::MinLatency,
        _                   => RoutingStrategy::Balanced,
    };

    let routing = RoutingHints {
        strategy,
        max_cost_per_1m: ext.max_cost_per_1m,
        max_latency_ms: ext.max_latency_ms,
        allow_providers: ext.allow_providers.unwrap_or_default(),
        deny_providers: ext.deny_providers.unwrap_or_default(),
        ..Default::default()
    };

    let mut request = Request::new(messages);
    request.model = body.model;
    request.routing = routing;
    request.params.stream = body.stream;
    if let Some(t) = body.temperature { request.params.temperature = Some(t); }
    if let Some(t) = body.max_tokens  { request.params.max_tokens  = Some(t); }
    if let Some(t) = body.top_p       { request.params.top_p       = Some(t); }

    Ok(request)
}
