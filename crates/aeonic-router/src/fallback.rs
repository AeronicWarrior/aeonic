use crate::{registry::ProviderRegistry, selector};
use aeonic_core::{
    error::{AeonicError, Result},
    traits::StreamResponse,
    types::{Request, Response},
};
use std::sync::Arc;
use tracing::{info, warn};

const DEFAULT_MAX_ATTEMPTS: u32 = 3;

/// Execute a request with automatic fallback.
///
/// Tries the best provider first. If it fails with a retryable error,
/// moves to the next best provider. Repeats up to `max_attempts`.
pub async fn execute_with_fallback(
    request: &Request,
    registry: &Arc<ProviderRegistry>,
    max_attempts: u32,
) -> Result<Response> {
    let max = max_attempts.max(1).min(10);
    let mut tried: Vec<String> = Vec::new();
    let mut last_error: Option<AeonicError> = None;

    for attempt in 1..=max {
        // Select next best provider, excluding already tried ones
        let selection = if tried.is_empty() {
            match selector::select(request, registry) {
                Ok(s) => s,
                Err(e) => return Err(e),
            }
        } else {
            match selector::select_fallback(request, registry, &tried) {
                Some(candidate) => crate::selector::Selection {
                    provider_id: candidate.model.provider.clone(),
                    model: candidate.model,
                },
                None => {
                    return Err(last_error.unwrap_or_else(|| {
                        AeonicError::Routing("All providers exhausted".into())
                    }));
                }
            }
        };

        let provider_id = selection.provider_id.clone();
        tried.push(provider_id.clone());

        let provider = match registry.get(&provider_id) {
            Some(p) => p,
            None => {
                warn!("Provider '{provider_id}' disappeared from registry during fallback");
                continue;
            }
        };

        // Patch the request's model to the selected model
        let mut patched = request.clone();
        patched.model = Some(selection.model.id.clone());

        info!(
            attempt,
            provider = %provider_id,
            model = %selection.model.id,
            "Attempting request"
        );

        match provider.complete(&patched).await {
            Ok(mut response) => {
                if attempt > 1 {
                    response.metadata.insert(
                        "fallback_attempt".into(),
                        serde_json::json!(attempt),
                    );
                    response.metadata.insert(
                        "tried_providers".into(),
                        serde_json::json!(tried),
                    );
                }
                return Ok(response);
            }
            Err(e) => {
                if e.is_retryable() {
                    warn!(
                        attempt,
                        provider = %provider_id,
                        error = %e,
                        "Retryable error — trying next provider"
                    );
                    last_error = Some(e);
                    // Small backoff before next attempt
                    if attempt < max {
                        tokio::time::sleep(
                            std::time::Duration::from_millis(200 * attempt as u64)
                        ).await;
                    }
                } else {
                    // Non-retryable (policy violation, auth error, etc) — bail immediately
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AeonicError::Routing(format!("Failed after {max} attempts"))
    }))
}

/// Streaming variant — falls back on connection failure only.
/// Once streaming starts, fallback is not possible.
pub async fn stream_with_fallback(
    request: &Request,
    registry: &Arc<ProviderRegistry>,
    max_attempts: u32,
) -> Result<StreamResponse> {
    let max = max_attempts.max(1).min(DEFAULT_MAX_ATTEMPTS);
    let mut tried: Vec<String> = Vec::new();
    let mut last_error: Option<AeonicError> = None;

    for attempt in 1..=max {
        let selection = if tried.is_empty() {
            selector::select(request, registry)?
        } else {
            match selector::select_fallback(request, registry, &tried) {
                Some(c) => crate::selector::Selection {
                    provider_id: c.model.provider.clone(),
                    model: c.model,
                },
                None => {
                    return Err(last_error.unwrap_or_else(|| {
                        AeonicError::Routing("All stream providers exhausted".into())
                    }));
                }
            }
        };

        let provider_id = selection.provider_id.clone();
        tried.push(provider_id.clone());

        let provider = match registry.get(&provider_id) {
            Some(p) => p,
            None => continue,
        };

        let mut patched = request.clone();
        patched.model = Some(selection.model.id.clone());
        patched.params.stream = true;

        info!(attempt, provider = %provider_id, "Attempting stream");

        match provider.stream(&patched).await {
            Ok(stream) => return Ok(stream),
            Err(e) if e.is_retryable() => {
                warn!(attempt, provider = %provider_id, error = %e, "Stream fallback");
                last_error = Some(e);
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AeonicError::Routing("Stream failed after all attempts".into())
    }))
}
