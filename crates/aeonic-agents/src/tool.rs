use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use async_trait::async_trait;
use aeonic_core::error::Result;

/// Definition of a tool the agent can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

impl Tool {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// A tool call requested by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

/// The result of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub tool_name: String,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ToolResult {
    pub fn success(call_id: impl Into<String>, tool_name: impl Into<String>, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            output,
            error: None,
            duration_ms,
        }
    }

    pub fn error(call_id: impl Into<String>, tool_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            output: serde_json::Value::Null,
            error: Some(error.into()),
            duration_ms: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// Trait for executable tools.
#[async_trait]
pub trait ToolExecutor: Send + Sync + 'static {
    fn tool(&self) -> Tool;
    async fn execute(&self, arguments: HashMap<String, serde_json::Value>) -> Result<serde_json::Value>;
}

/// Built-in tool: web search (stub — wire to a real search API).
pub struct WebSearchTool;

#[async_trait]
impl ToolExecutor for WebSearchTool {
    fn tool(&self) -> Tool {
        Tool::new(
            "web_search",
            "Search the web for current information",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"]
            }),
        )
    }

    async fn execute(&self, arguments: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let query = arguments.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Stub — in production, call a real search API
        Ok(serde_json::json!({
            "results": [
                { "title": format!("Search results for: {query}"), "snippet": "Results would appear here with a real search API." }
            ]
        }))
    }
}

/// Built-in tool: code execution (stub).
pub struct CodeExecutorTool;

#[async_trait]
impl ToolExecutor for CodeExecutorTool {
    fn tool(&self) -> Tool {
        Tool::new(
            "execute_code",
            "Execute Python code and return the output",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Python code to execute" },
                    "language": { "type": "string", "enum": ["python", "javascript"] }
                },
                "required": ["code"]
            }),
        )
    }

    async fn execute(&self, arguments: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let code = arguments.get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Stub — in production, run in a sandbox
        Ok(serde_json::json!({
            "output": format!("Code execution stub. Would run: {}", &code[..code.len().min(100)]),
            "exit_code": 0
        }))
    }
}
