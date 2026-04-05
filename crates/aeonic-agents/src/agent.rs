use aeonic_core::{
    error::Result,
    types::{Message, Request, Response},
};
use aeonic_router::AeonicRouter;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Configuration for an agent — its identity, instructions, and routing preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique name for this agent.
    pub name: String,
    /// System prompt — the agent's instructions and persona.
    pub system_prompt: String,
    /// Model to use. If None, the router auto-selects.
    pub model: Option<String>,
    /// Routing strategy: "balanced", "min_cost", "max_quality", "min_latency".
    pub routing_strategy: String,
    /// Max tokens for responses.
    pub max_tokens: u32,
    /// Temperature (0.0–1.0). Lower = more deterministic.
    pub temperature: f32,
    /// Arbitrary metadata attached to this agent.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentConfig {
    pub fn new(name: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: system_prompt.into(),
            model: None,
            routing_strategy: "balanced".into(),
            max_tokens: 4096,
            temperature: 0.7,
            metadata: HashMap::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.routing_strategy = strategy.into();
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = tokens;
        self
    }
}

/// The result of an agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: Uuid,
    pub agent_name: String,
    pub content: String,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub latency_ms: u64,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentResponse {
    pub fn from_response(agent_name: &str, response: Response) -> Self {
        Self {
            id: response.id,
            agent_name: agent_name.to_string(),
            content: response.message.text().to_string(),
            model: response.model,
            provider: response.provider,
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            latency_ms: response.latency_ms,
            created_at: response.created_at,
            metadata: response.metadata,
        }
    }
}

/// Core trait every agent implements.
#[async_trait]
pub trait Agent: Send + Sync + 'static {
    fn config(&self) -> &AgentConfig;
    fn name(&self) -> &str { &self.config().name }

    /// Run the agent with a user message and optional conversation history.
    async fn run(
        &self,
        input: &str,
        history: &[Message],
    ) -> Result<AgentResponse>;

    /// Build the request for this agent.
    fn build_request(&self, input: &str, history: &[Message]) -> Request {
        let config = self.config();
        let mut messages = vec![Message::system(&config.system_prompt)];
        messages.extend_from_slice(history);
        messages.push(Message::user(input));

        let mut request = Request::new(messages);
        request.model = config.model.clone();
        request.params.temperature = Some(config.temperature);
        request.params.max_tokens = Some(config.max_tokens);
        request.routing.strategy = match config.routing_strategy.as_str() {
            "min_cost"    => aeonic_core::types::RoutingStrategy::MinCost,
            "max_quality" => aeonic_core::types::RoutingStrategy::MaxQuality,
            "min_latency" => aeonic_core::types::RoutingStrategy::MinLatency,
            _             => aeonic_core::types::RoutingStrategy::Balanced,
        };
        request
    }
}

/// A simple base agent that wraps the router.
pub struct BaseAgent {
    config: AgentConfig,
    router: Arc<AeonicRouter>,
}

impl BaseAgent {
    pub fn new(config: AgentConfig, router: Arc<AeonicRouter>) -> Self {
        Self { config, router }
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn config(&self) -> &AgentConfig { &self.config }

    async fn run(&self, input: &str, history: &[Message]) -> Result<AgentResponse> {
        use aeonic_core::traits::Router;
        let request = self.build_request(input, history);
        let response = self.router.route(request).await?;
        Ok(AgentResponse::from_response(&self.config.name, response))
    }
}
