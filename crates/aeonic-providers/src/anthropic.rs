use aeonic_core::{
    error::{AeonicError, Result},
    traits::{Provider, StreamResponse},
    types::{
        CapabilityTier, FinishReason, Message, MessageContent, MessageRole,
        ModelInfo, Request, Response, StreamChunk, TokenUsage,
    },
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, instrument};
use uuid::Uuid;

const BASE_URL: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build reqwest client"),
            api_key: api_key.into(),
            base_url: BASE_URL.into(),
        }
    }

    fn model_id_or_default<'a>(&self, request: &'a Request) -> &'a str {
        request.model.as_deref().unwrap_or("claude-haiku-4-5-20251001")
    }

    fn split_messages(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let system = messages
            .iter()
            .find(|m| matches!(m.role, MessageRole::System))
            .map(|m| m.text().to_string());
        let msgs = messages
            .iter()
            .filter(|m| !matches!(m.role, MessageRole::System))
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User      => "user",
                    MessageRole::Assistant => "assistant",
                    _                      => "user",
                }.to_string(),
                content: match &m.content {
                    MessageContent::Text(t) => t.clone(),
                    MessageContent::Parts(_) => String::new(),
                },
            })
            .collect();
        (system, msgs)
    }

    fn build_body(&self, request: &Request, stream: bool) -> AnthropicRequest {
        let (system, messages) = Self::split_messages(&request.messages);
        let p = &request.params;
        AnthropicRequest {
            model: self.model_id_or_default(request).to_string(),
            messages,
            system,
            max_tokens: p.max_tokens.unwrap_or(4096),
            temperature: p.temperature,
            top_p: p.top_p,
            stream: Some(stream),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str { "anthropic" }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "claude-opus-4-5".into(),
                provider: "anthropic".into(),
                display_name: "Claude Opus 4.5".into(),
                capability: CapabilityTier::Frontier,
                context_window: 200_000,
                max_output_tokens: 32_000,
                cost_input_per_1m: 15.00,
                cost_output_per_1m: 75.00,
                supports_streaming: true,
                supports_vision: true,
                supports_tools: true,
                supports_json_mode: true,
            },
            ModelInfo {
                id: "claude-sonnet-4-5".into(),
                provider: "anthropic".into(),
                display_name: "Claude Sonnet 4.5".into(),
                capability: CapabilityTier::Advanced,
                context_window: 200_000,
                max_output_tokens: 16_000,
                cost_input_per_1m: 3.00,
                cost_output_per_1m: 15.00,
                supports_streaming: true,
                supports_vision: true,
                supports_tools: true,
                supports_json_mode: true,
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".into(),
                provider: "anthropic".into(),
                display_name: "Claude Haiku 4.5".into(),
                capability: CapabilityTier::Standard,
                context_window: 200_000,
                max_output_tokens: 8_192,
                cost_input_per_1m: 0.80,
                cost_output_per_1m: 4.00,
                supports_streaming: true,
                supports_vision: true,
                supports_tools: true,
                supports_json_mode: true,
            },
        ]
    }

    async fn health_check(&self) -> bool { true }

    #[instrument(skip(self, request), fields(provider = "anthropic", model = ?request.model))]
    async fn complete(&self, request: &Request) -> Result<Response> {
        let started = Instant::now();
        let model_id = self.model_id_or_default(request).to_string();
        debug!("Sending request to Anthropic: model={model_id}");

        let http_resp = self.client
            .post(format!("{}/messages", self.base_url))
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&self.build_body(request, false))
            .send()
            .await
            .map_err(|e| AeonicError::Http(e.to_string()))?;

        if !http_resp.status().is_success() {
            let status = http_resp.status().as_u16();
            let text = http_resp.text().await.unwrap_or_default();
            return match status {
                401 => Err(AeonicError::Auth { provider: "anthropic".into() }),
                429 => Err(AeonicError::RateLimit { provider: "anthropic".into(), retry_after_secs: 60 }),
                _ => Err(AeonicError::provider("anthropic", format!("HTTP {status}: {text}"))),
            };
        }

        let raw: AnthropicResponse = http_resp.json().await
            .map_err(|e| AeonicError::provider("anthropic", e.to_string()))?;

        let text = raw.content.into_iter()
            .filter_map(|c| if c.content_type == "text" { Some(c.text) } else { None })
            .collect::<Vec<_>>().join("");

        Ok(Response {
            id: Uuid::new_v4(),
            request_id: request.id,
            created_at: chrono::Utc::now(),
            message: Message::assistant(text),
            model: model_id,
            provider: "anthropic".into(),
            usage: TokenUsage::new(raw.usage.input_tokens, raw.usage.output_tokens),
            latency_ms: started.elapsed().as_millis() as u64,
            finish_reason: match raw.stop_reason.as_deref() {
                Some("end_turn")   => FinishReason::Stop,
                Some("max_tokens") => FinishReason::Length,
                _                  => FinishReason::Stop,
            },
            metadata: Default::default(),
        })
    }

    #[instrument(skip(self, request), fields(provider = "anthropic", model = ?request.model))]
    async fn stream(&self, request: &Request) -> Result<StreamResponse> {
        let model_id = self.model_id_or_default(request).to_string();
        let request_id = request.id;

        let http_resp = self.client
            .post(format!("{}/messages", self.base_url))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "text/event-stream")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&self.build_body(request, true))
            .send()
            .await
            .map_err(|e| AeonicError::Http(e.to_string()))?;

        if !http_resp.status().is_success() {
            let status = http_resp.status().as_u16();
            let text = http_resp.text().await.unwrap_or_default();
            return Err(AeonicError::provider("anthropic", format!("HTTP {status}: {text}")));
        }

        Ok(Box::pin(parse_anthropic_sse(http_resp.bytes_stream(), model_id, request_id)))
    }
}

fn parse_anthropic_sse(
    byte_stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static,
    model: String,
    request_id: Uuid,
) -> impl Stream<Item = Result<StreamChunk>> + Send {
    async_stream::stream! {
        let mut buffer = String::new();
        tokio::pin!(byte_stream);
        while let Some(chunk) = byte_stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => { yield Err(AeonicError::Stream(e.to_string())); return; }
            };
            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(pos) = buffer.find("\n\n") {
                let event = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 2..].to_string();
                let mut event_type = String::new();
                let mut data = String::new();
                for line in event.lines() {
                    if let Some(t) = line.strip_prefix("event: ") { event_type = t.trim().to_string(); }
                    else if let Some(d) = line.strip_prefix("data: ") { data = d.trim().to_string(); }
                }
                match event_type.as_str() {
                    "content_block_delta" => {
                        if let Ok(ev) = serde_json::from_str::<AnthropicDeltaEvent>(&data) {
                            yield Ok(StreamChunk {
                                request_id,
                                delta: ev.delta.text.unwrap_or_default(),
                                model: model.clone(),
                                provider: "anthropic".into(),
                                finish_reason: None,
                                usage: None,
                            });
                        }
                    }
                    "message_stop" => return,
                    "error" => { yield Err(AeonicError::Stream(data)); return; }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct AnthropicMessage { role: String, content: String }

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage { input_tokens: u32, output_tokens: u32 }

#[derive(Deserialize)]
struct AnthropicDeltaEvent { delta: AnthropicDelta }

#[derive(Deserialize)]
struct AnthropicDelta { text: Option<String> }
