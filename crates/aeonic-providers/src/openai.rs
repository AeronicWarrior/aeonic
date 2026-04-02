use aeonic_core::{
    error::{AeonicError, Result},
    traits::{Provider, StreamResponse},
    types::{
        CapabilityTier, FinishReason, Message, MessageContent, MessageRole,
        ModelInfo, ModelParams, Request, Response, StreamChunk, TokenUsage,
    },
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::{pin::Pin, time::Instant};
use tracing::{debug, instrument, warn};
use uuid::Uuid;

const BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, BASE_URL)
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    fn model_id_or_default<'a>(&self, request: &'a Request) -> &'a str {
        request
            .model
            .as_deref()
            .unwrap_or("gpt-4o-mini")
    }

    fn build_chat_request(&self, request: &Request) -> OpenAiChatRequest {
        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(|m| OpenAiMessage {
                role: match m.role {
                    MessageRole::System    => "system",
                    MessageRole::User      => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool      => "tool",
                }
                .to_string(),
                content: match &m.content {
                    MessageContent::Text(t) => t.clone(),
                    MessageContent::Parts(_) => String::new(),
                },
            })
            .collect();

        let p = &request.params;
        OpenAiChatRequest {
            model: self.model_id_or_default(request).to_string(),
            messages,
            max_tokens: p.max_tokens,
            temperature: p.temperature,
            top_p: p.top_p,
            stop: p.stop.clone(),
            stream: Some(p.stream),
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4o".into(),
                provider: "openai".into(),
                display_name: "GPT-4o".into(),
                capability: CapabilityTier::Frontier,
                context_window: 128_000,
                max_output_tokens: 16_384,
                cost_input_per_1m: 2.50,
                cost_output_per_1m: 10.00,
                supports_streaming: true,
                supports_vision: true,
                supports_tools: true,
                supports_json_mode: true,
            },
            ModelInfo {
                id: "gpt-4o-mini".into(),
                provider: "openai".into(),
                display_name: "GPT-4o Mini".into(),
                capability: CapabilityTier::Standard,
                context_window: 128_000,
                max_output_tokens: 16_384,
                cost_input_per_1m: 0.15,
                cost_output_per_1m: 0.60,
                supports_streaming: true,
                supports_vision: true,
                supports_tools: true,
                supports_json_mode: true,
            },
            ModelInfo {
                id: "o1".into(),
                provider: "openai".into(),
                display_name: "o1".into(),
                capability: CapabilityTier::Frontier,
                context_window: 200_000,
                max_output_tokens: 100_000,
                cost_input_per_1m: 15.00,
                cost_output_per_1m: 60.00,
                supports_streaming: false,
                supports_vision: true,
                supports_tools: true,
                supports_json_mode: false,
            },
            ModelInfo {
                id: "o3-mini".into(),
                provider: "openai".into(),
                display_name: "o3 Mini".into(),
                capability: CapabilityTier::Advanced,
                context_window: 200_000,
                max_output_tokens: 100_000,
                cost_input_per_1m: 1.10,
                cost_output_per_1m: 4.40,
                supports_streaming: false,
                supports_vision: false,
                supports_tools: true,
                supports_json_mode: true,
            },
        ]
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/models", self.base_url))
            .header(header::AUTHORIZATION, self.auth_header())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    #[instrument(skip(self, request), fields(provider = "openai", model = ?request.model))]
    async fn complete(&self, request: &Request) -> Result<Response> {
        let started = Instant::now();
        let model_id = self.model_id_or_default(request).to_string();

        let mut chat_req = self.build_chat_request(request);
        chat_req.stream = Some(false);

        debug!("Sending request to OpenAI: model={}", model_id);

        let http_resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header(header::AUTHORIZATION, self.auth_header())
            .header(header::CONTENT_TYPE, "application/json")
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| AeonicError::Http(e.to_string()))?;

        if !http_resp.status().is_success() {
            let status = http_resp.status().as_u16();
            let body = http_resp.text().await.unwrap_or_default();
            return match status {
                401 => Err(AeonicError::Auth { provider: "openai".into() }),
                429 => Err(AeonicError::RateLimit { provider: "openai".into(), retry_after_secs: 60 }),
                _ => Err(AeonicError::provider("openai", format!("HTTP {status}: {body}"))),
            };
        }

        let raw: OpenAiChatResponse = http_resp
            .json()
            .await
            .map_err(|e| AeonicError::provider("openai", e.to_string()))?;

        let choice = raw.choices.into_iter().next()
            .ok_or_else(|| AeonicError::provider("openai", "empty choices array"))?;

        let usage = raw.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            cost_usd: None,
        }).unwrap_or_default();

        let latency_ms = started.elapsed().as_millis() as u64;

        Ok(Response {
            id: Uuid::new_v4(),
            request_id: request.id,
            created_at: chrono::Utc::now(),
            message: Message::assistant(choice.message.content),
            model: model_id,
            provider: "openai".into(),
            usage,
            latency_ms,
            finish_reason: match choice.finish_reason.as_str() {
                "stop"           => FinishReason::Stop,
                "length"         => FinishReason::Length,
                "tool_calls"     => FinishReason::ToolCalls,
                "content_filter" => FinishReason::ContentFilter,
                _                => FinishReason::Stop,
            },
            metadata: Default::default(),
        })
    }

    #[instrument(skip(self, request), fields(provider = "openai", model = ?request.model))]
    async fn stream(&self, request: &Request) -> Result<StreamResponse> {
        let model_id = self.model_id_or_default(request).to_string();
        let request_id = request.id;

        let mut chat_req = self.build_chat_request(request);
        chat_req.stream = Some(true);

        let http_resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header(header::AUTHORIZATION, self.auth_header())
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "text/event-stream")
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| AeonicError::Http(e.to_string()))?;

        if !http_resp.status().is_success() {
            let status = http_resp.status().as_u16();
            let body = http_resp.text().await.unwrap_or_default();
            return Err(AeonicError::provider("openai", format!("HTTP {status}: {body}")));
        }

        let byte_stream = http_resp.bytes_stream();

        let stream = parse_sse_stream(byte_stream, model_id, request_id);
        Ok(Box::pin(stream))
    }
}

fn parse_sse_stream(
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
                Err(e) => {
                    yield Err(AeonicError::Stream(e.to_string()));
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(pos) = buffer.find("\n\n") {
                let event = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return;
                        }
                        match serde_json::from_str::<OpenAiStreamChunk>(data) {
                            Ok(chunk) => {
                                let delta = chunk
                                    .choices
                                    .first()
                                    .and_then(|c| c.delta.content.as_deref())
                                    .unwrap_or("")
                                    .to_string();

                                let finish_reason = chunk
                                    .choices
                                    .first()
                                    .and_then(|c| c.finish_reason.as_deref())
                                    .map(|r| match r {
                                        "stop"   => FinishReason::Stop,
                                        "length" => FinishReason::Length,
                                        _        => FinishReason::Stop,
                                    });

                                yield Ok(StreamChunk {
                                    request_id,
                                    delta,
                                    model: model.clone(),
                                    provider: "openai".into(),
                                    finish_reason,
                                    usage: None,
                                });
                            }
                            Err(e) => {
                                warn!("Failed to parse SSE chunk: {e} — data: {data}");
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── OpenAI wire types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
    finish_reason: String,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}
