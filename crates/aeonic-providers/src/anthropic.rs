use aeonic_core::{
    error::Result,
    traits::{Provider, StreamResponse},
    types::{CapabilityTier, ModelInfo, Request, Response},
};
use async_trait::async_trait;
use reqwest::Client;

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
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

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

    async fn health_check(&self) -> bool {
        // Anthropic has no public /models endpoint — do a lightweight ping
        true
    }

    async fn complete(&self, _request: &Request) -> Result<Response> {
        // Full implementation coming in Phase 1
        todo!("Anthropic complete() — coming in Phase 1")
    }

    async fn stream(&self, _request: &Request) -> Result<StreamResponse> {
        todo!("Anthropic stream() — coming in Phase 1")
    }
}
