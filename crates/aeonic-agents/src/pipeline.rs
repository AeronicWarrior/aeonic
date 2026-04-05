use crate::agent::{Agent, AgentResponse};
use aeonic_core::{error::Result, types::Message};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// How a pipeline step feeds into the next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepInput {
    /// Pass the original user input unchanged.
    Original,
    /// Pass the output of the previous step.
    PreviousOutput,
    /// Use a custom template. Use `{input}` and `{previous}` as placeholders.
    Template(String),
}

/// A single step in a pipeline.
pub struct PipelineStep {
    pub name: String,
    pub agent: Arc<dyn Agent>,
    pub input_mode: StepInput,
}

impl PipelineStep {
    pub fn new(name: impl Into<String>, agent: Arc<dyn Agent>) -> Self {
        Self {
            name: name.into(),
            agent,
            input_mode: StepInput::PreviousOutput,
        }
    }

    pub fn with_input_mode(mut self, mode: StepInput) -> Self {
        self.input_mode = mode;
        self
    }
}

/// The result of running a full pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineResult {
    pub id: Uuid,
    pub original_input: String,
    pub final_output: String,
    pub steps: Vec<AgentResponse>,
    pub total_prompt_tokens: u32,
    pub total_completion_tokens: u32,
    pub total_latency_ms: u64,
    pub created_at: chrono::DateTime<Utc>,
}

impl PipelineResult {
    pub fn total_tokens(&self) -> u32 {
        self.total_prompt_tokens + self.total_completion_tokens
    }

    /// Get the output of a specific step by agent name.
    pub fn step_output(&self, agent_name: &str) -> Option<&str> {
        self.steps
            .iter()
            .find(|s| s.agent_name == agent_name)
            .map(|s| s.content.as_str())
    }
}

/// A sequential pipeline that chains agents one after another.
/// The output of each step becomes the input of the next.
pub struct Pipeline {
    pub name: String,
    steps: Vec<PipelineStep>,
}

impl Pipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
        }
    }

    /// Add a step to the pipeline.
    pub fn step(mut self, step: PipelineStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add an agent directly as a step (uses PreviousOutput input mode).
    pub fn then(mut self, agent: Arc<dyn Agent>) -> Self {
        let name = agent.name().to_string();
        self.steps.push(PipelineStep::new(name, agent));
        self
    }

    /// Run the pipeline end-to-end.
    #[instrument(skip(self), fields(pipeline = %self.name))]
    pub async fn run(&self, input: &str) -> Result<PipelineResult> {
        let started = std::time::Instant::now();
        let pipeline_id = Uuid::new_v4();

        info!(pipeline = %self.name, steps = self.steps.len(), "Starting pipeline");

        let mut step_results: Vec<AgentResponse> = Vec::new();
        let mut current_output = input.to_string();

        for (i, step) in self.steps.iter().enumerate() {
            let step_input = match &step.input_mode {
                StepInput::Original => input.to_string(),
                StepInput::PreviousOutput => current_output.clone(),
                StepInput::Template(tmpl) => tmpl
                    .replace("{input}", input)
                    .replace("{previous}", &current_output),
            };

            // Build conversation history from previous steps
            let history: Vec<Message> = step_results
                .iter()
                .flat_map(|r| vec![
                    Message::user(&r.content),
                    Message::assistant(&r.content),
                ])
                .collect();

            info!(
                pipeline = %self.name,
                step = %step.name,
                step_num = i + 1,
                total_steps = self.steps.len(),
                "Running pipeline step"
            );

            let response = step.agent.run(&step_input, &history).await?;
            current_output = response.content.clone();
            step_results.push(response);
        }

        let total_prompt = step_results.iter().map(|r| r.prompt_tokens).sum();
        let total_completion = step_results.iter().map(|r| r.completion_tokens).sum();

        info!(
            pipeline = %self.name,
            total_tokens = total_prompt + total_completion,
            latency_ms = started.elapsed().as_millis(),
            "Pipeline complete"
        );

        Ok(PipelineResult {
            id: pipeline_id,
            original_input: input.to_string(),
            final_output: current_output,
            steps: step_results,
            total_prompt_tokens: total_prompt,
            total_completion_tokens: total_completion,
            total_latency_ms: started.elapsed().as_millis() as u64,
            created_at: Utc::now(),
        })
    }
}

/// A parallel pipeline that runs multiple agents simultaneously
/// and combines their outputs.
pub struct ParallelPipeline {
    pub name: String,
    agents: Vec<Arc<dyn Agent>>,
}

impl ParallelPipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), agents: Vec::new() }
    }

    pub fn agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.agents.push(agent);
        self
    }

    /// Run all agents in parallel and return all their responses.
    #[instrument(skip(self), fields(pipeline = %self.name))]
    pub async fn run(&self, input: &str) -> Result<Vec<AgentResponse>> {
        use futures::future::join_all;

        info!(
            pipeline = %self.name,
            agents = self.agents.len(),
            "Starting parallel pipeline"
        );

        let futures: Vec<_> = self.agents
            .iter()
            .map(|agent| {
                let input = input.to_string();
                let agent = Arc::clone(agent);
                async move { agent.run(&input, &[]).await }
            })
            .collect();

        let results = join_all(futures).await;

        // Collect successes, surface first error if all fail
        let mut responses = Vec::new();
        let mut last_err = None;
        for r in results {
            match r {
                Ok(resp) => responses.push(resp),
                Err(e) => last_err = Some(e),
            }
        }

        if responses.is_empty() {
            return Err(last_err.unwrap_or_else(|| {
                aeonic_core::error::AeonicError::Agent("All parallel agents failed".into())
            }));
        }

        info!(
            pipeline = %self.name,
            succeeded = responses.len(),
            "Parallel pipeline complete"
        );

        Ok(responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_result_total_tokens() {
        let result = PipelineResult {
            id: Uuid::new_v4(),
            original_input: "test".into(),
            final_output: "output".into(),
            steps: vec![],
            total_prompt_tokens: 100,
            total_completion_tokens: 50,
            total_latency_ms: 1000,
            created_at: Utc::now(),
        };
        assert_eq!(result.total_tokens(), 150);
    }
}
