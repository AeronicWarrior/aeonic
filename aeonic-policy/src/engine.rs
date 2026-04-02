use crate::rules::{default_rules, PolicyDecision, PolicyRule};
use aeonic_core::{
    error::{AeonicError, Result},
    types::{ModelInfo, Request},
};
use tracing::{debug, warn};

/// The policy engine evaluates a set of rules against every request
/// before it reaches a provider. Deny rules short-circuit immediately.
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// Create an engine with the built-in default rules.
    pub fn default_rules() -> Self {
        Self {
            rules: default_rules(),
        }
    }

    /// Create an engine with a custom rule set.
    pub fn new(rules: Vec<PolicyRule>) -> Self {
        Self { rules }
    }

    /// Create an engine with no rules (allow everything).
    pub fn permissive() -> Self {
        Self { rules: vec![] }
    }

    /// Add a rule at runtime.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    /// Evaluate all rules against a request+model pair.
    /// Returns Ok(()) if the request is allowed, Err if denied.
    pub fn evaluate(&self, request: &Request, model: Option<&ModelInfo>) -> Result<()> {
        for rule in &self.rules {
            match rule.evaluate(request, model) {
                PolicyDecision::Allow => {
                    debug!(rule = %rule.name, "Policy: allow");
                }
                PolicyDecision::Warn { message } => {
                    warn!(rule = %rule.name, "{message}");
                }
                PolicyDecision::Deny { reason } => {
                    return Err(AeonicError::PolicyViolation {
                        rule: rule.name.clone(),
                        detail: reason,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::default_rules()
    }
}
