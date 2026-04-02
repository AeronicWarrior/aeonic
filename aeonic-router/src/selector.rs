use crate::{registry::ProviderRegistry, scorer::ScoredModel};
use aeonic_core::{
    error::{AeonicError, Result},
    types::{ModelInfo, Request},
};
use std::sync::Arc;
use tracing::debug;

/// The result of a selection — a provider + model ready to execute.
pub struct Selection {
    pub provider_id: String,
    pub model: ModelInfo,
}

/// Select the best provider+model for a request.
///
/// If the request explicitly names a model, we honour it.
/// Otherwise we score all available models and pick the top candidate.
pub fn select(request: &Request, registry: &Arc<ProviderRegistry>) -> Result<Selection> {
    // 1. Explicit model requested — find its provider directly
    if let Some(model_id) = &request.model {
        return select_explicit(model_id, registry);
    }

    // 2. Auto-select based on routing hints
    select_auto(request, registry)
}

fn select_explicit(model_id: &str, registry: &Arc<ProviderRegistry>) -> Result<Selection> {
    let model = registry.model_info(model_id).ok_or_else(|| AeonicError::NoProvider {
        model: model_id.to_string(),
        reason: "no registered provider supports this model".into(),
    })?;

    debug!("Explicit model selection: {} via {}", model.id, model.provider);

    Ok(Selection {
        provider_id: model.provider.clone(),
        model,
    })
}

fn select_auto(request: &Request, registry: &Arc<ProviderRegistry>) -> Result<Selection> {
    let all_models = registry.all_models();

    if all_models.is_empty() {
        return Err(AeonicError::Routing(
            "No providers registered. Add at least one provider to the registry.".into(),
        ));
    }

    let scored = crate::scorer::score_models(all_models, &request.routing);

    let best = scored.into_iter().next().ok_or_else(|| AeonicError::Routing(
        "No model passed the routing constraints. Try relaxing cost ceiling or capability floor.".into(),
    ))?;

    debug!(
        "Auto-selected model: {} (provider: {}, score: {:.1})",
        best.model.id, best.model.provider, best.score
    );

    Ok(Selection {
        provider_id: best.model.provider.clone(),
        model: best.model,
    })
}

/// Pick the next best model for fallback, excluding already-tried providers.
pub fn select_fallback(
    request: &Request,
    registry: &Arc<ProviderRegistry>,
    exclude_providers: &[String],
) -> Option<ScoredModel> {
    let all_models = registry.all_models();
    let mut scored = crate::scorer::score_models(all_models, &request.routing);
    scored.retain(|s| !exclude_providers.contains(&s.model.provider));
    scored.into_iter().next()
}
