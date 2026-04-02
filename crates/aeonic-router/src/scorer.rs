use aeonic_core::types::{
    CapabilityTier, ModelInfo, RoutingHints, RoutingStrategy, TaskType,
};

/// A scored candidate model ready for selection.
#[derive(Debug, Clone)]
pub struct ScoredModel {
    pub model: ModelInfo,
    /// Higher score = better fit for this request.
    pub score: f64,
}

/// Score all candidate models against the routing hints.
/// Returns candidates sorted best-first, filtered against hard constraints.
pub fn score_models(models: Vec<ModelInfo>, hints: &RoutingHints) -> Vec<ScoredModel> {
    let mut scored: Vec<ScoredModel> = models
        .into_iter()
        .filter(|m| passes_hard_constraints(m, hints))
        .map(|m| {
            let score = compute_score(&m, hints);
            ScoredModel { model: m, score }
        })
        .collect();

    // Sort best score first
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

/// Hard constraints — any model failing these is immediately excluded.
fn passes_hard_constraints(model: &ModelInfo, hints: &RoutingHints) -> bool {
    // Capability floor
    if model.capability < hints.min_capability {
        return false;
    }

    // Cost ceiling (input cost as proxy)
    if let Some(max_cost) = hints.max_cost_per_1m {
        if model.cost_input_per_1m > max_cost {
            return false;
        }
    }

    // Explicit provider deny list
    if hints.deny_providers.contains(&model.provider) {
        return false;
    }

    // Explicit provider allow list (if set, only allow listed providers)
    if !hints.allow_providers.is_empty() && !hints.allow_providers.contains(&model.provider) {
        return false;
    }

    // Task-specific requirements
    if let Some(task) = &hints.task_type {
        match task {
            TaskType::Vision => {
                if !model.supports_vision {
                    return false;
                }
            }
            TaskType::Embedding => {
                // Embeddings need a separate endpoint — filter out chat models for now
                return false;
            }
            _ => {}
        }
    }

    true
}

/// Soft scoring — produces a 0.0–100.0 score.
/// Higher is better. Components are weighted by routing strategy.
fn compute_score(model: &ModelInfo, hints: &RoutingHints) -> f64 {
    let cost_score    = cost_score(model);
    let quality_score = quality_score(model);
    let latency_score = latency_score(model);

    match hints.strategy {
        RoutingStrategy::MinCost => {
            cost_score * 0.70 + quality_score * 0.20 + latency_score * 0.10
        }
        RoutingStrategy::MaxQuality => {
            quality_score * 0.70 + latency_score * 0.20 + cost_score * 0.10
        }
        RoutingStrategy::MinLatency => {
            latency_score * 0.70 + cost_score * 0.20 + quality_score * 0.10
        }
        RoutingStrategy::Balanced => {
            quality_score * 0.40 + cost_score * 0.35 + latency_score * 0.25
        }
    }
}

/// Cost score: cheaper models score higher.
/// Uses input cost per 1M tokens as the proxy.
/// Score range: 0–100.
fn cost_score(model: &ModelInfo) -> f64 {
    // Free (local) models get max score
    if model.cost_input_per_1m == 0.0 {
        return 100.0;
    }
    // Scale: $0.10/1M → 95, $1/1M → 75, $5/1M → 40, $20/1M → 0
    let score = 100.0 - (model.cost_input_per_1m.ln() + 5.0) * 12.0;
    score.clamp(0.0, 100.0)
}

/// Quality score based on capability tier.
fn quality_score(model: &ModelInfo) -> f64 {
    let base = match model.capability {
        CapabilityTier::Basic    => 30.0,
        CapabilityTier::Standard => 55.0,
        CapabilityTier::Advanced => 78.0,
        CapabilityTier::Frontier => 100.0,
    };

    // Bonus for feature support
    let mut bonus: f64 = 0.0;
    if model.supports_tools      { bonus += 3.0; }
    if model.supports_vision     { bonus += 3.0; }
    if model.supports_json_mode  { bonus += 2.0; }
    if model.context_window >= 100_000 { bonus += 4.0; }

    (base + bonus).min(100.0)
}

/// Latency score: smaller models and local providers score higher.
fn latency_score(model: &ModelInfo) -> f64 {
    // Local providers (Ollama) have near-zero network latency overhead
    if model.cost_input_per_1m == 0.0 {
        return 85.0;
    }

    // Proxy latency from cost tier — cheaper models tend to be faster
    match model.capability {
        CapabilityTier::Basic    => 90.0,
        CapabilityTier::Standard => 75.0,
        CapabilityTier::Advanced => 55.0,
        CapabilityTier::Frontier => 35.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeonic_core::types::{RoutingHints, RoutingStrategy};

    fn make_model(id: &str, cost: f64, cap: CapabilityTier) -> ModelInfo {
        ModelInfo {
            id: id.into(),
            provider: "test".into(),
            display_name: id.into(),
            capability: cap,
            context_window: 128_000,
            max_output_tokens: 4_096,
            cost_input_per_1m: cost,
            cost_output_per_1m: cost * 4.0,
            supports_streaming: true,
            supports_vision: false,
            supports_tools: true,
            supports_json_mode: true,
        }
    }

    #[test]
    fn min_cost_strategy_picks_cheapest() {
        let models = vec![
            make_model("expensive", 15.0, CapabilityTier::Frontier),
            make_model("cheap",     0.15, CapabilityTier::Standard),
            make_model("mid",       3.0,  CapabilityTier::Advanced),
        ];
        let hints = RoutingHints {
            strategy: RoutingStrategy::MinCost,
            ..Default::default()
        };
        let scored = score_models(models, &hints);
        assert_eq!(scored[0].model.id, "cheap");
    }

    #[test]
    fn max_quality_picks_frontier() {
        let models = vec![
            make_model("frontier", 15.0, CapabilityTier::Frontier),
            make_model("standard", 0.15, CapabilityTier::Standard),
        ];
        let hints = RoutingHints {
            strategy: RoutingStrategy::MaxQuality,
            ..Default::default()
        };
        let scored = score_models(models, &hints);
        assert_eq!(scored[0].model.id, "frontier");
    }

    #[test]
    fn cost_ceiling_filters_expensive_models() {
        let models = vec![
            make_model("expensive", 15.0, CapabilityTier::Frontier),
            make_model("cheap",     0.15, CapabilityTier::Standard),
        ];
        let hints = RoutingHints {
            max_cost_per_1m: Some(1.0),
            ..Default::default()
        };
        let scored = score_models(models, &hints);
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].model.id, "cheap");
    }

    #[test]
    fn capability_floor_filters_weak_models() {
        let models = vec![
            make_model("basic",    0.05, CapabilityTier::Basic),
            make_model("frontier", 15.0, CapabilityTier::Frontier),
        ];
        let hints = RoutingHints {
            min_capability: CapabilityTier::Advanced,
            ..Default::default()
        };
        let scored = score_models(models, &hints);
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].model.id, "frontier");
    }
}
