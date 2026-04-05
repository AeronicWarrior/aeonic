use crate::{
    agent::{Agent, AgentConfig, AgentResponse, BaseAgent},
    pipeline::{ParallelPipeline, PipelineResult, Pipeline, PipelineStep, StepInput},
    worker::WorkerAgent,
    critic::CriticAgent,
};
use aeonic_core::{
    error::Result,
    traits::Router,
    types::Message,
};
use aeonic_router::AeonicRouter;
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use tracing::{info, instrument};

/// The Orchestrator is the top-level agent that:
/// 1. Receives a complex task
/// 2. Breaks it into subtasks (plan)
/// 3. Dispatches subtasks to worker agents in parallel
/// 4. Synthesizes the results into a final answer
/// 5. Optionally passes through a critic for quality verification
pub struct OrchestratorAgent {
    config: AgentConfig,
    router: Arc<AeonicRouter>,
    max_workers: usize,
    use_critic: bool,
}

impl OrchestratorAgent {
    pub fn new(router: Arc<AeonicRouter>) -> Self {
        let config = AgentConfig::new(
            "orchestrator",
            "You are an expert task orchestrator. When given a complex task, \
             break it down into 2-4 clear, independent subtasks. \
             Respond ONLY with a JSON array of subtask strings. \
             Example: [\"Research X\", \"Analyze Y\", \"Summarize Z\"]",
        )
        .with_strategy("max_quality")
        .with_temperature(0.3);

        Self {
            config,
            router,
            max_workers: 4,
            use_critic: true,
        }
    }

    pub fn with_max_workers(mut self, n: usize) -> Self {
        self.max_workers = n;
        self
    }

    pub fn with_critic(mut self, enabled: bool) -> Self {
        self.use_critic = enabled;
        self
    }

    /// Full orchestration: plan → parallel workers → synthesize → (optional) critique
    #[instrument(skip(self), fields(agent = "orchestrator"))]
    pub async fn orchestrate(&self, task: &str) -> Result<OrchestratedResult> {
        let started = std::time::Instant::now();
        info!(task = %&task[..task.len().min(80)], "Starting orchestration");

        // Step 1: Plan — break task into subtasks
        let plan_response = self.run(task, &[]).await?;
        let subtasks = self.parse_subtasks(&plan_response.content);
        info!(subtasks = subtasks.len(), "Task decomposed");

        // Step 2: Parallel workers — execute each subtask
        let mut parallel = ParallelPipeline::new("workers");
        for (i, subtask) in subtasks.iter().enumerate().take(self.max_workers) {
            let worker_config = AgentConfig::new(
                format!("worker-{}", i + 1),
                format!(
                    "You are a focused expert. Complete this specific subtask thoroughly and concisely:\n\n{}",
                    subtask
                ),
            )
            .with_strategy("balanced")
            .with_temperature(0.5);

            let worker = Arc::new(WorkerAgent::new(
                worker_config,
                Arc::clone(&self.router),
                subtask.clone(),
            ));
            parallel = parallel.agent(worker as Arc<dyn Agent>);
        }

        let worker_responses = parallel.run(task).await?;

        // Step 3: Synthesize all worker outputs
        let synthesis_context = worker_responses
            .iter()
            .enumerate()
            .map(|(i, r)| format!("## Result {} ({})\n{}", i + 1, r.agent_name, r.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let synthesis_prompt = format!(
            "Original task: {}\n\nWorker results:\n{}\n\nSynthesize these results into a comprehensive, coherent final answer.",
            task, synthesis_context
        );

        let synthesizer_config = AgentConfig::new(
            "synthesizer",
            "You are an expert at synthesizing multiple pieces of information into clear, comprehensive answers.",
        )
        .with_strategy("max_quality")
        .with_temperature(0.4);

        let synthesizer = BaseAgent::new(synthesizer_config, Arc::clone(&self.router));
        let synthesis_response = synthesizer.run(&synthesis_prompt, &[]).await?;

        // Step 4: Optional critic pass
        let (final_output, critique) = if self.use_critic {
            let critic = CriticAgent::new(Arc::clone(&self.router));
            let critique_response = critic.run(
                &format!("Task: {}\n\nAnswer: {}", task, synthesis_response.content),
                &[],
            ).await?;
            let content = synthesis_response.content.clone();
            (content, Some(critique_response))
        } else {
            (synthesis_response.content.clone(), None)
        };

        let total_tokens: u32 = worker_responses.iter().map(|r| r.prompt_tokens + r.completion_tokens).sum::<u32>()
            + plan_response.prompt_tokens + plan_response.completion_tokens
            + synthesis_response.prompt_tokens + synthesis_response.completion_tokens
            + critique.as_ref().map(|c| c.prompt_tokens + c.completion_tokens).unwrap_or(0);

        info!(
            total_tokens,
            latency_ms = started.elapsed().as_millis(),
            "Orchestration complete"
        );

        Ok(OrchestratedResult {
            task: task.to_string(),
            subtasks,
            worker_responses,
            synthesis: synthesis_response,
            critique,
            final_output,
            total_tokens,
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }

    fn parse_subtasks(&self, content: &str) -> Vec<String> {
        // Try to parse as JSON array first
        let clean = content.trim();
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(clean) {
            return arr;
        }

        // Try to find JSON array in the text
        if let Some(start) = clean.find('[') {
            if let Some(end) = clean.rfind(']') {
                let json_str = &clean[start..=end];
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(json_str) {
                    return arr;
                }
            }
        }

        // Fallback: split by newlines and treat numbered items as subtasks
        clean
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| {
                // Strip leading numbers/bullets
                l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '*' || c == ' ')
                    .to_string()
            })
            .filter(|l| !l.is_empty())
            .take(4)
            .collect()
    }
}

#[async_trait]
impl Agent for OrchestratorAgent {
    fn config(&self) -> &AgentConfig { &self.config }

    async fn run(&self, input: &str, history: &[Message]) -> Result<AgentResponse> {
        use aeonic_core::traits::Router;
        let request = self.build_request(input, history);
        let response = self.router.route(request).await?;
        Ok(AgentResponse::from_response(&self.config.name, response))
    }
}

/// The full result of an orchestrated task.
#[derive(Debug)]
pub struct OrchestratedResult {
    pub task: String,
    pub subtasks: Vec<String>,
    pub worker_responses: Vec<AgentResponse>,
    pub synthesis: AgentResponse,
    pub critique: Option<AgentResponse>,
    pub final_output: String,
    pub total_tokens: u32,
    pub latency_ms: u64,
}

impl OrchestratedResult {
    /// Print a summary of the orchestration to stdout.
    pub fn print_summary(&self) {
        println!("\n=== Orchestration Summary ===");
        println!("Task: {}", &self.task[..self.task.len().min(80)]);
        println!("Subtasks: {}", self.subtasks.len());
        for (i, st) in self.subtasks.iter().enumerate() {
            println!("  {}. {}", i + 1, st);
        }
        println!("Workers completed: {}", self.worker_responses.len());
        println!("Total tokens: {}", self.total_tokens);
        println!("Total latency: {}ms", self.latency_ms);
        println!("\n=== Final Output ===");
        println!("{}", self.final_output);
        if let Some(c) = &self.critique {
            println!("\n=== Critique ===");
            println!("{}", c.content);
        }
    }
}
