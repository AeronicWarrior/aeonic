use aeonic_core::types::{ModelInfo, Request};
use serde::{Deserialize, Serialize};

/// The verdict returned after evaluating a policy rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Request is allowed to proceed.
    Allow,
    /// Request is denied with a reason.
    Deny { reason: String },
    /// Request is allowed but with a warning annotation.
    Warn { message: String },
}

impl PolicyDecision {
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }
}

/// A single declarative policy rule.
/// Rules are evaluated in order; first Deny wins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub enabled: bool,
    pub condition: RuleCondition,
    pub action: RuleAction,
}

impl PolicyRule {
    pub fn evaluate(&self, request: &Request, model: Option<&ModelInfo>) -> PolicyDecision {
        if !self.enabled {
            return PolicyDecision::Allow;
        }

        if self.condition.matches(request, model) {
            match &self.action {
                RuleAction::Deny { reason } => PolicyDecision::Deny {
                    reason: reason.clone(),
                },
                RuleAction::Warn { message } => PolicyDecision::Warn {
                    message: message.clone(),
                },
                RuleAction::Allow => PolicyDecision::Allow,
            }
        } else {
            PolicyDecision::Allow
        }
    }
}

/// Conditions that trigger a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleCondition {
    /// True when estimated cost exceeds threshold (USD per 1M tokens).
    CostExceeds { usd_per_1m: f64 },
    /// True when requested token count exceeds limit.
    TokensExceed { max_tokens: u32 },
    /// True when provider is in the blocked list.
    ProviderBlocked { providers: Vec<String> },
    /// True when model is in the blocked list.
    ModelBlocked { models: Vec<String> },
    /// True when message count exceeds limit (large context abuse).
    MessageCountExceeds { count: usize },
    /// True when any message content contains a blocked keyword.
    ContentContains { keywords: Vec<String> },
    /// Always true — use for catch-all rules.
    Always,
}

impl RuleCondition {
    pub fn matches(&self, request: &Request, model: Option<&ModelInfo>) -> bool {
        match self {
            Self::CostExceeds { usd_per_1m } => {
                model.map_or(false, |m| m.cost_input_per_1m > *usd_per_1m)
            }
            Self::TokensExceed { max_tokens } => {
                request.params.max_tokens.map_or(false, |t| t > *max_tokens)
            }
            Self::ProviderBlocked { providers } => {
                model.map_or(false, |m| providers.contains(&m.provider))
            }
            Self::ModelBlocked { models } => {
                model.map_or(false, |m| models.contains(&m.id))
            }
            Self::MessageCountExceeds { count } => {
                request.messages.len() > *count
            }
            Self::ContentContains { keywords } => {
                request.messages.iter().any(|msg| {
                    let text = msg.text().to_lowercase();
                    keywords.iter().any(|kw| text.contains(kw.as_str()))
                })
            }
            Self::Always => true,
        }
    }
}

/// Actions taken when a condition matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleAction {
    Allow,
    Deny { reason: String },
    Warn { message: String },
}

/// Built-in default rules. Applied when no custom policy file is provided.
pub fn default_rules() -> Vec<PolicyRule> {
    vec![
        PolicyRule {
            name: "block-runaway-cost".into(),
            enabled: true,
            condition: RuleCondition::CostExceeds { usd_per_1m: 100.0 },
            action: RuleAction::Deny {
                reason: "Model cost exceeds the $100/1M token ceiling".into(),
            },
        },
        PolicyRule {
            name: "block-token-abuse".into(),
            enabled: true,
            condition: RuleCondition::TokensExceed { max_tokens: 100_000 },
            action: RuleAction::Deny {
                reason: "Requested max_tokens exceeds the 100k per-request limit".into(),
            },
        },
        PolicyRule {
            name: "warn-large-context".into(),
            enabled: true,
            condition: RuleCondition::MessageCountExceeds { count: 100 },
            action: RuleAction::Warn {
                message: "Request has >100 messages — consider summarising context".into(),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use aeonic_core::types::{CapabilityTier, Message, ModelInfo, Request};

    fn make_model(cost: f64) -> ModelInfo {
        ModelInfo {
            id: "test-model".into(),
            provider: "test".into(),
            display_name: "Test".into(),
            capability: CapabilityTier::Standard,
            context_window: 128_000,
            max_output_tokens: 4_096,
            cost_input_per_1m: cost,
            cost_output_per_1m: cost * 4.0,
            supports_streaming: true,
            supports_vision: false,
            supports_tools: false,
            supports_json_mode: false,
        }
    }

    #[test]
    fn cost_rule_denies_expensive_model() {
        let rule = PolicyRule {
            name: "cost-check".into(),
            enabled: true,
            condition: RuleCondition::CostExceeds { usd_per_1m: 10.0 },
            action: RuleAction::Deny { reason: "Too expensive".into() },
        };

        let request = Request::new(vec![Message::user("hello")]);
        let model = make_model(50.0);

        assert!(rule.evaluate(&request, Some(&model)).is_denied());
    }

    #[test]
    fn cost_rule_allows_cheap_model() {
        let rule = PolicyRule {
            name: "cost-check".into(),
            enabled: true,
            condition: RuleCondition::CostExceeds { usd_per_1m: 10.0 },
            action: RuleAction::Deny { reason: "Too expensive".into() },
        };

        let request = Request::new(vec![Message::user("hello")]);
        let model = make_model(0.15);

        assert!(!rule.evaluate(&request, Some(&model)).is_denied());
    }

    #[test]
    fn disabled_rule_always_allows() {
        let rule = PolicyRule {
            name: "disabled".into(),
            enabled: false,
            condition: RuleCondition::Always,
            action: RuleAction::Deny { reason: "Should not fire".into() },
        };
        let request = Request::new(vec![Message::user("hello")]);
        assert_eq!(rule.evaluate(&request, None), PolicyDecision::Allow);
    }
}
