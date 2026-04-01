use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Role of a message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            name: None,
            tool_call_id: None,
        }
    }

    pub fn text(&self) -> &str {
        match &self.content {
            MessageContent::Text(t) => t,
            MessageContent::Parts(_) => "",
        }
    }
}

/// Message content — plain text or multimodal parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

/// A part of a multimodal message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A routing + inference request — the core input to Aeonic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Unique ID for this request.
    pub id: Uuid,
    /// Timestamp of request creation.
    pub created_at: DateTime<Utc>,
    /// Conversation history.
    pub messages: Vec<Message>,
    /// Preferred model. If None, router selects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Routing hints and constraints.
    pub routing: RoutingHints,
    /// Model parameters.
    pub params: ModelParams,
    /// Arbitrary metadata (for tracing, policy, etc).
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Request {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            messages,
            model: None,
            routing: RoutingHints::default(),
            params: ModelParams::default(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.messages.insert(0, Message::system(system));
        self
    }
}

/// Hints to the router about how to select a provider/model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingHints {
    /// Optimize for: cost, latency, quality, or balanced.
    pub strategy: RoutingStrategy,
    /// Max cost in USD per 1M tokens (input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cost_per_1m: Option<f64>,
    /// Max acceptable latency in ms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_latency_ms: Option<u64>,
    /// Minimum capability tier required.
    pub min_capability: CapabilityTier,
    /// Explicitly allow or deny providers.
    #[serde(default)]
    pub allow_providers: Vec<String>,
    #[serde(default)]
    pub deny_providers: Vec<String>,
    /// Task type hint for semantic routing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<TaskType>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    #[default]
    Balanced,
    MinCost,
    MinLatency,
    MaxQuality,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityTier {
    #[default]
    Basic,
    Standard,
    Advanced,
    Frontier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Coding,
    Reasoning,
    Writing,
    Summarization,
    Classification,
    Extraction,
    Conversation,
    Math,
    Vision,
    Embedding,
}

/// Parameters passed directly to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    pub stream: bool,
}

impl Default for ModelParams {
    fn default() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: None,
            stop: None,
            stream: false,
        }
    }
}

/// A completed response from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: Uuid,
    pub request_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub message: Message,
    pub model: String,
    pub provider: String,
    pub usage: TokenUsage,
    pub latency_ms: u64,
    pub finish_reason: FinishReason,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A chunk from a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub request_id: Uuid,
    pub delta: String,
    pub model: String,
    pub provider: String,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error,
}

/// Token usage accounting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// Estimated cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

impl TokenUsage {
    pub fn new(prompt: u32, completion: u32) -> Self {
        Self {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: prompt + completion,
            cost_usd: None,
        }
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost_usd = Some(cost);
        self
    }
}

/// Metadata describing a model available from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub capability: CapabilityTier,
    pub context_window: usize,
    pub max_output_tokens: usize,
    /// Cost per 1M input tokens in USD.
    pub cost_input_per_1m: f64,
    /// Cost per 1M output tokens in USD.
    pub cost_output_per_1m: f64,
    pub supports_streaming: bool,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_json_mode: bool,
}

impl ModelInfo {
    pub fn estimate_cost(&self, usage: &TokenUsage) -> f64 {
        let input_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * self.cost_input_per_1m;
        let output_cost = (usage.completion_tokens as f64 / 1_000_000.0) * self.cost_output_per_1m;
        input_cost + output_cost
    }
}

/// Supported LLM provider kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    Mistral,
    Groq,
    Ollama,
    Bedrock,
    Custom(String),
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAi => write!(f, "openai"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Gemini => write!(f, "gemini"),
            Self::Mistral => write!(f, "mistral"),
            Self::Groq => write!(f, "groq"),
            Self::Ollama => write!(f, "ollama"),
            Self::Bedrock => write!(f, "bedrock"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}
