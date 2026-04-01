use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level Aeonic configuration.
/// Loaded from `aeonic.toml` or environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AeonicConfig {
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub max_request_body_bytes: usize,
    pub request_timeout_ms: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8080,
            max_request_body_bytes: 10 * 1024 * 1024, // 10 MB
            request_timeout_ms: 120_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Default routing strategy if not specified per-request.
    pub default_strategy: String,
    /// Maximum number of fallback attempts.
    pub max_fallback_attempts: u32,
    /// Whether to enable automatic model selection.
    pub auto_select_model: bool,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_strategy: "balanced".into(),
            max_fallback_attempts: 3,
            auto_select_model: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: None,
            base_url: None,
            timeout_ms: Some(60_000),
            max_retries: Some(3),
            extra: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub otlp_endpoint: Option<String>,
    pub log_level: String,
    pub log_format: LogFormat,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            otlp_endpoint: None,
            log_level: "info".into(),
            log_format: LogFormat::Pretty,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub enabled: bool,
    /// Path to a .rego policy file. If None, default policy is used.
    pub policy_file: Option<String>,
    /// Hard cap on cost per request in USD.
    pub max_cost_per_request_usd: Option<f64>,
    /// Hard cap on tokens per request.
    pub max_tokens_per_request: Option<u32>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            policy_file: None,
            max_cost_per_request_usd: None,
            max_tokens_per_request: None,
        }
    }
}
