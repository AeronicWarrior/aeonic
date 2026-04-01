use thiserror::Error;

pub type Result<T> = std::result::Result<T, AeonicError>;

#[derive(Debug, Error)]
pub enum AeonicError {
    #[error("Provider error from '{provider}': {message}")]
    Provider {
        provider: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Routing failed: {0}")]
    Routing(String),

    #[error("No provider available for model '{model}': {reason}")]
    NoProvider { model: String, reason: String },

    #[error("Policy violation: {rule} — {detail}")]
    PolicyViolation { rule: String, detail: String },

    #[error("Rate limit exceeded for provider '{provider}'. Retry after {retry_after_secs}s")]
    RateLimit {
        provider: String,
        retry_after_secs: u64,
    },

    #[error("Authentication failed for provider '{provider}'")]
    Auth { provider: String },

    #[error("Request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Token limit exceeded: requested {requested}, limit {limit}")]
    TokenLimit { requested: usize, limit: usize },

    #[error("Context too large: {tokens} tokens exceeds model max {max_tokens}")]
    ContextTooLarge { tokens: usize, max_tokens: usize },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("State store error: {0}")]
    State(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unsupported feature '{feature}' for provider '{provider}'")]
    Unsupported { feature: String, provider: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AeonicError {
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
            source: None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. } | Self::Timeout { .. } | Self::Http(_)
        )
    }

    pub fn is_policy_violation(&self) -> bool {
        matches!(self, Self::PolicyViolation { .. })
    }
}
