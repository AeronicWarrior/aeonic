use crate::agent::{Agent, AgentConfig, AgentResponse};
use aeonic_core::{error::Result, traits::Router, types::Message};
use aeonic_router::AeonicRouter;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::instrument;

/// A worker agent focused on executing one specific subtask.
/// Workers are typically spawned by the Orchestrator.
pub struct WorkerAgent {
    config: AgentConfig,
    router: Arc<AeonicRouter>,
    /// The specific subtask this worker is assigned to.
    pub subtask: String,
}

impl WorkerAgent {
    pub fn new(config: AgentConfig, router: Arc<AeonicRouter>, subtask: String) -> Self {
        Self { config, router, subtask }
    }

    /// Create a worker with a default config for a given subtask.
    pub fn for_subtask(subtask: impl Into<String>, router: Arc<AeonicRouter>) -> Self {
        let subtask = subtask.into();
        let config = AgentConfig::new(
            "worker",
            format!(
                "You are a focused, expert assistant. Your job is to complete \
                 the following subtask thoroughly and accurately:\n\n{}",
                subtask
            ),
        )
        .with_strategy("balanced")
        .with_temperature(0.5);

        Self { config, router, subtask }
    }

    /// Create a coding-focused worker.
    pub fn coder(subtask: impl Into<String>, router: Arc<AeonicRouter>) -> Self {
        let subtask = subtask.into();
        let config = AgentConfig::new(
            "coder-worker",
            format!(
                "You are an expert software engineer. Write clean, well-commented, \
                 production-quality code for the following task:\n\n{}",
                subtask
            ),
        )
        .with_strategy("max_quality")
        .with_temperature(0.2);

        Self { config, router, subtask }
    }

    /// Create a research-focused worker.
    pub fn researcher(subtask: impl Into<String>, router: Arc<AeonicRouter>) -> Self {
        let subtask = subtask.into();
        let config = AgentConfig::new(
            "research-worker",
            format!(
                "You are a thorough research analyst. Provide detailed, \
                 well-structured analysis for the following:\n\n{}",
                subtask
            ),
        )
        .with_strategy("max_quality")
        .with_temperature(0.4);

        Self { config, router, subtask }
    }
}

#[async_trait]
impl Agent for WorkerAgent {
    fn config(&self) -> &AgentConfig { &self.config }

    #[instrument(skip(self, history), fields(agent = %self.config.name, subtask = %&self.subtask[..self.subtask.len().min(60)]))]
    async fn run(&self, input: &str, history: &[Message]) -> Result<AgentResponse> {
        let request = self.build_request(input, history);
        let response = self.router.route(request).await?;
        Ok(AgentResponse::from_response(&self.config.name, response))
    }
}
