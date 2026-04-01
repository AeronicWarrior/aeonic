use crate::{
    error::Result,
    types::{ModelInfo, Request, Response, StreamChunk},
};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// A streaming response — pinned boxed async stream of chunks.
pub type StreamResponse = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Core trait every LLM provider must implement.
///
/// Providers are stateless — all config lives in the struct fields.
/// The router selects a provider, then calls complete() or stream().
#[async_trait]
pub trait Provider: Send + Sync + 'static {
    /// Unique identifier for this provider (e.g. "openai", "anthropic").
    fn id(&self) -> &str;

    /// List all models available from this provider.
    fn models(&self) -> Vec<ModelInfo>;

    /// Fetch a specific model's info by ID.
    fn model(&self, id: &str) -> Option<ModelInfo> {
        self.models().into_iter().find(|m| m.id == id)
    }

    /// Whether this provider is currently healthy (can accept requests).
    async fn health_check(&self) -> bool;

    /// Execute a non-streaming completion.
    async fn complete(&self, request: &Request) -> Result<Response>;

    /// Execute a streaming completion.
    /// Returns a stream of delta chunks.
    async fn stream(&self, request: &Request) -> Result<StreamResponse>;

    /// Count tokens for a request without sending it.
    /// Returns None if the provider doesn't support token counting.
    async fn count_tokens(&self, request: &Request) -> Option<u32> {
        let _ = request;
        None
    }
}

/// Core trait the router must implement.
///
/// The router selects the best provider+model for a request,
/// applies policy, handles fallbacks, and returns a response.
#[async_trait]
pub trait Router: Send + Sync + 'static {
    /// Route a request and return a complete response.
    async fn route(&self, request: Request) -> Result<Response>;

    /// Route a request and return a stream.
    async fn route_stream(&self, request: Request) -> Result<StreamResponse>;

    /// List all registered providers.
    fn providers(&self) -> Vec<&dyn Provider>;

    /// List all models across all registered providers.
    fn models(&self) -> Vec<ModelInfo> {
        self.providers()
            .iter()
            .flat_map(|p| p.models())
            .collect()
    }
}

/// Core trait for multi-step agents.
///
/// An agent receives a request, may make multiple LLM calls
/// and tool invocations, then returns a final response.
#[async_trait]
pub trait Agent: Send + Sync + 'static {
    /// Unique name for this agent.
    fn name(&self) -> &str;

    /// Description of what this agent does.
    fn description(&self) -> &str;

    /// Execute the agent and return a final response.
    async fn run(&self, request: Request) -> Result<Response>;
}

/// Core trait for state / memory stores.
///
/// Used by agents to persist context across turns or sessions.
#[async_trait]
pub trait StateStore: Send + Sync + 'static {
    /// Store a value by key.
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()>;

    /// Retrieve a value by key.
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;

    /// Delete a key.
    async fn delete(&self, key: &str) -> Result<()>;

    /// List all keys with an optional prefix.
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>>;
}

/// A middleware layer that wraps a Provider.
///
/// Used for retry logic, rate limiting, cost tracking, etc.
#[async_trait]
pub trait ProviderMiddleware: Send + Sync + 'static {
    async fn on_request(&self, request: &mut Request) -> Result<()>;
    async fn on_response(&self, response: &mut Response) -> Result<()>;
}
