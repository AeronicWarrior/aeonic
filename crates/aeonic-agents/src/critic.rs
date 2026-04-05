use crate::agent::{Agent, AgentConfig, AgentResponse};
use aeonic_core::{error::Result, traits::Router, types::Message};
use aeonic_router::AeonicRouter;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// A critic agent that evaluates the quality of another agent's output.
/// Used as the final verification step in orchestrated pipelines.
pub struct CriticAgent {
    config: AgentConfig,
    router: Arc<AeonicRouter>,
}

impl CriticAgent {
    pub fn new(router: Arc<AeonicRouter>) -> Self {
        let config = AgentConfig::new(
            "critic",
            "You are a rigorous quality critic. Review the given task and answer. \
             Evaluate: accuracy, completeness, clarity, and usefulness. \
             Respond with a JSON object: \
             {\"score\": 1-10, \"strengths\": [\"...\"], \"weaknesses\": [\"...\"], \"verdict\": \"pass|fail\", \"suggestion\": \"...\"}",
        )
        .with_strategy("max_quality")
        .with_temperature(0.2);

        Self { config, router }
    }

    /// Run critique and parse the structured result.
    pub async fn critique(&self, task: &str, answer: &str) -> Result<CritiqueResult> {
        let prompt = format!("Task: {}\n\nAnswer to evaluate:\n{}", task, answer);
        let response = self.run(&prompt, &[]).await?;
        Ok(CritiqueResult::parse(&response))
    }
}

#[async_trait]
impl Agent for CriticAgent {
    fn config(&self) -> &AgentConfig { &self.config }

    #[instrument(skip(self, history), fields(agent = "critic"))]
    async fn run(&self, input: &str, history: &[Message]) -> Result<AgentResponse> {
        let request = self.build_request(input, history);
        let response = self.router.route(request).await?;
        Ok(AgentResponse::from_response(&self.config.name, response))
    }
}

/// Structured critique output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueResult {
    pub raw_response: AgentResponse,
    pub score: Option<u8>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub verdict: CritiqueVerdict,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CritiqueVerdict {
    Pass,
    Fail,
    Unknown,
}

impl CritiqueResult {
    fn parse(response: &AgentResponse) -> Self {
        let content = response.content.trim();

        // Try to extract JSON from response
        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                &content[start..=end]
            } else { content }
        } else { content };

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
            let score = v["score"].as_u64().map(|s| s as u8);
            let strengths = v["strengths"].as_array()
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let weaknesses = v["weaknesses"].as_array()
                .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let verdict = match v["verdict"].as_str() {
                Some("pass") => CritiqueVerdict::Pass,
                Some("fail") => CritiqueVerdict::Fail,
                _            => CritiqueVerdict::Unknown,
            };
            let suggestion = v["suggestion"].as_str().map(String::from);

            return Self {
                raw_response: response.clone(),
                score,
                strengths,
                weaknesses,
                verdict,
                suggestion,
            };
        }

        // Fallback if JSON parse fails
        Self {
            raw_response: response.clone(),
            score: None,
            strengths: vec![],
            weaknesses: vec![],
            verdict: CritiqueVerdict::Unknown,
            suggestion: Some(content.to_string()),
        }
    }

    pub fn passed(&self) -> bool {
        self.verdict == CritiqueVerdict::Pass
            || self.score.map(|s| s >= 7).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_critique_json() {
        use uuid::Uuid;
        use chrono::Utc;

        let response = AgentResponse {
            id: Uuid::new_v4(),
            agent_name: "critic".into(),
            content: r#"{"score": 8, "strengths": ["clear", "accurate"], "weaknesses": ["brief"], "verdict": "pass", "suggestion": "Add more examples"}"#.into(),
            model: "test".into(),
            provider: "test".into(),
            prompt_tokens: 10,
            completion_tokens: 20,
            latency_ms: 100,
            created_at: Utc::now(),
            metadata: Default::default(),
        };

        let result = CritiqueResult::parse(&response);
        assert_eq!(result.score, Some(8));
        assert_eq!(result.verdict, CritiqueVerdict::Pass);
        assert!(result.passed());
        assert_eq!(result.strengths.len(), 2);
    }
}
