use crate::metrics::{CostSummary, ProviderStats, RequestMetrics};
use aeonic_core::types::{Request, Response};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tracing::{info, instrument};
use uuid::Uuid;

/// Records every request/response and provides aggregated metrics.
/// Thread-safe — clone freely across tasks.
#[derive(Clone)]
pub struct TelemetryRecorder {
    inner: Arc<Inner>,
}

struct Inner {
    /// Per-request records (last N requests for recent history)
    records: DashMap<Uuid, RequestMetrics>,
    /// Per-provider running stats
    provider_stats: DashMap<String, ProviderStatsAccumulator>,
    /// Global counters
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_tokens: AtomicU64,
    total_cost_micros: AtomicU64, // stored as micro-dollars to avoid float atomics
}

#[derive(Default)]
struct ProviderStatsAccumulator {
    requests: u64,
    errors: u64,
    tokens: u64,
    cost_micros: u64,
    latency_sum_ms: u64,
}

impl TelemetryRecorder {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                records: DashMap::new(),
                provider_stats: DashMap::new(),
                total_requests: AtomicU64::new(0),
                total_errors: AtomicU64::new(0),
                total_tokens: AtomicU64::new(0),
                total_cost_micros: AtomicU64::new(0),
            }),
        }
    }

    /// Record a completed request/response pair.
    #[instrument(skip(self, request, response), fields(
        request_id = %request.id,
        provider = %response.provider,
        model = %response.model,
        latency_ms = response.latency_ms,
        tokens = response.usage.total_tokens,
    ))]
    pub fn record(&self, request: &Request, response: &Response) {
        let cost = response.usage.cost_usd.unwrap_or(0.0);

        info!(
            provider = %response.provider,
            model = %response.model,
            tokens = response.usage.total_tokens,
            latency_ms = response.latency_ms,
            cost_usd = cost,
            "Request completed"
        );

        // Global counters
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);
        self.inner.total_tokens
            .fetch_add(response.usage.total_tokens as u64, Ordering::Relaxed);
        self.inner.total_cost_micros
            .fetch_add((cost * 1_000_000.0) as u64, Ordering::Relaxed);

        // Per-provider stats
        let mut stats = self.inner.provider_stats
            .entry(response.provider.clone())
            .or_default();
        stats.requests += 1;
        stats.tokens += response.usage.total_tokens as u64;
        stats.cost_micros += (cost * 1_000_000.0) as u64;
        stats.latency_sum_ms += response.latency_ms;
        drop(stats);

        // Store full record
        let metrics = RequestMetrics {
            request_id: request.id,
            recorded_at: Utc::now(),
            provider: response.provider.clone(),
            model: response.model.clone(),
            prompt_tokens: response.usage.prompt_tokens,
            completion_tokens: response.usage.completion_tokens,
            total_tokens: response.usage.total_tokens,
            cost_usd: response.usage.cost_usd,
            latency_ms: response.latency_ms,
            success: true,
        };
        self.inner.records.insert(request.id, metrics);
    }

    /// Record a failed request.
    pub fn record_error(&self, request: &Request, provider: &str, error: &str) {
        self.inner.total_requests.fetch_add(1, Ordering::Relaxed);
        self.inner.total_errors.fetch_add(1, Ordering::Relaxed);

        let mut stats = self.inner.provider_stats
            .entry(provider.to_string())
            .or_default();
        stats.requests += 1;
        stats.errors += 1;
        drop(stats);

        tracing::error!(
            request_id = %request.id,
            provider,
            error,
            "Request failed"
        );
    }

    /// Total requests handled (success + error).
    pub fn total_requests(&self) -> u64 {
        self.inner.total_requests.load(Ordering::Relaxed)
    }

    /// Total tokens across all requests.
    pub fn total_tokens(&self) -> u64 {
        self.inner.total_tokens.load(Ordering::Relaxed)
    }

    /// Total estimated cost in USD.
    pub fn total_cost_usd(&self) -> f64 {
        self.inner.total_cost_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    /// Cost summary across all requests.
    pub fn cost_summary(&self) -> CostSummary {
        CostSummary {
            total_requests: self.total_requests(),
            total_tokens: self.total_tokens(),
            total_cost_usd: self.total_cost_usd(),
            total_errors: self.inner.total_errors.load(Ordering::Relaxed),
        }
    }

    /// Per-provider statistics.
    pub fn provider_stats(&self) -> Vec<ProviderStats> {
        self.inner
            .provider_stats
            .iter()
            .map(|entry| {
                let acc = entry.value();
                let avg_latency = if acc.requests > 0 {
                    acc.latency_sum_ms / acc.requests
                } else {
                    0
                };
                ProviderStats {
                    provider: entry.key().clone(),
                    requests: acc.requests,
                    errors: acc.errors,
                    tokens: acc.tokens,
                    cost_usd: acc.cost_micros as f64 / 1_000_000.0,
                    avg_latency_ms: avg_latency,
                }
            })
            .collect()
    }

    /// Look up a specific request record.
    pub fn get_record(&self, request_id: &Uuid) -> Option<RequestMetrics> {
        self.inner.records.get(request_id).map(|r| r.clone())
    }
}

impl Default for TelemetryRecorder {
    fn default() -> Self {
        Self::new()
    }
}
