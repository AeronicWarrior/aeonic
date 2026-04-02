use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Full record for a single request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetrics {
    pub request_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: Option<f64>,
    pub latency_ms: u64,
    pub success: bool,
}

/// Aggregate cost summary across all requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub total_errors: u64,
}

/// Per-provider aggregated statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub provider: String,
    pub requests: u64,
    pub errors: u64,
    pub tokens: u64,
    pub cost_usd: f64,
    pub avg_latency_ms: u64,
}
