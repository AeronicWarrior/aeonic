use aeonic_core::{
    error::Result,
    traits::{Provider, StreamResponse},
    types::{CapabilityTier, ModelInfo, Request, Response},
};
use async_trait::async_trait;
use reqwest::Client;

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to build reqwest client"),
            base_url: base_url.into(),
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
    }

    fn models(&self) -> Vec<ModelInfo> {
        // Ollama models are dynamic — in a full implementation
        // we'd fetch /api/tags. For now return common defaults.
        vec![
            ModelInfo {
                id: "llama3.2".into(),
                provider: "ollama".into(),
                display_name: "Llama 3.2 (local)".into(),
                capability: CapabilityTier::Standard,
                context_window: 128_000,
                max_output_tokens: 4_096,
                cost_input_per_1m: 0.0,
                cost_output_per_1m: 0.0,
                supports_streaming: true,
                supports_vision: false,
                supports_tools: true,
                supports_json_mode: true,
            },
            ModelInfo {
                id: "mistral".into(),
                provider: "ollama".into(),
                display_name: "Mistral 7B (local)".into(),
                capability: CapabilityTier::Basic,
                context_window: 32_000,
                max_output_tokens: 4_096,
                cost_input_per_1m: 0.0,
                cost_output_per_1m: 0.0,
                supports_streaming: true,
                supports_vision: false,
                supports_tools: false,
                supports_json_mode: true,
            },
        ]
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn complete(&self, _request: &Request) -> Result<Response> {
        todo!("Ollama complete() — coming in Phase 1")
    }

    async fn stream(&self, _request: &Request) -> Result<StreamResponse> {
        todo!("Ollama stream() — coming in Phase 1")
    }
}
