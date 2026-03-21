pub mod analyze;
pub mod document;
pub mod generate;
pub mod refactor;
pub mod test;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Permission level for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Tool can be executed freely
    Free,
    /// Tool requires user confirmation
    RequiresConfirmation,
    /// Tool is disabled
    Disabled,
}

/// Tool execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_duration_ms: u64,
}

impl ToolMetrics {
    pub fn new() -> Self {
        Self {
            executions: 0,
            successes: 0,
            failures: 0,
            total_duration_ms: 0,
        }
    }

    pub fn record(&mut self, success: bool, duration: Duration) {
        self.executions += 1;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
        self.total_duration_ms += duration.as_millis() as u64;
    }

    pub fn success_rate(&self) -> f64 {
        if self.executions == 0 {
            0.0
        } else {
            (self.successes as f64 / self.executions as f64) * 100.0
        }
    }

    pub fn avg_duration_ms(&self) -> u64 {
        if self.executions == 0 {
            0
        } else {
            self.total_duration_ms / self.executions
        }
    }
}

impl Default for ToolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
    pub duration_ms: Option<u64>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: HashMap::new(),
            duration_ms: None,
        }
    }

    pub fn success_with_duration(output: impl Into<String>, duration: Duration) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: HashMap::new(),
            duration_ms: Some(duration.as_millis() as u64),
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            metadata: HashMap::new(),
            duration_ms: None,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn formatted_output(&self) -> String {
        let mut output = String::new();

        if self.success {
            output.push_str("✓ Success\n");
            output.push_str(&self.output);
        } else {
            output.push_str("✗ Error\n");
            if let Some(err) = &self.error {
                output.push_str(err);
            }
        }

        if let Some(duration) = self.duration_ms {
            output.push_str(&format!("\n({}ms)", duration));
        }

        output
    }

    pub fn truncated(&self, max_len: usize) -> ToolResult {
        let mut result = self.clone();
        if result.output.len() > max_len {
            result.output = format!(
                "{}...\n[Output truncated - {} chars total]",
                &result.output[..max_len],
                self.output.len()
            );
        }
        result
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Free
    }
    fn usage(&self) -> &str {
        ""
    }
    async fn execute(&self, args: &str) -> Result<ToolResult>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    metrics: HashMap<String, ToolMetrics>,
    auto_confirm: bool,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            metrics: HashMap::new(),
            auto_confirm: false,
        }
    }

    pub fn with_auto_confirm(mut self, auto_confirm: bool) -> Self {
        self.auto_confirm = auto_confirm;
        self
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), Box::new(tool));
        self.metrics.insert(name, ToolMetrics::new());
    }

    pub async fn execute(&mut self, name: &str, args: &str) -> Result<ToolResult> {
        // Check if tool exists
        let tool = match self.tools.get(name) {
            Some(tool) => tool,
            None => return Ok(ToolResult::error(format!("Unknown tool: {}", name))),
        };

        // Check permission level
        let permission = tool.permission_level();
        if permission == PermissionLevel::Disabled {
            return Ok(ToolResult::error(format!("Tool '{}' is disabled", name)));
        }

        if permission == PermissionLevel::RequiresConfirmation && !self.auto_confirm {
            println!("Tool '{}' requires confirmation. Args: {}", name, args);
            print!("Execute? (y/n): ");
            use std::io::{self, Write};
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                return Ok(ToolResult::error("Execution cancelled by user".to_string()));
            }
        }

        // Execute and track metrics
        let start = Instant::now();
        let result = tool.execute(args).await;
        let duration = start.elapsed();

        match &result {
            Ok(tool_result) => {
                if let Some(metrics) = self.metrics.get_mut(name) {
                    metrics.record(tool_result.success, duration);
                }
                Ok(tool_result.clone())
            }
            Err(e) => {
                if let Some(metrics) = self.metrics.get_mut(name) {
                    metrics.record(false, duration);
                }
                Ok(ToolResult::error(format!("Tool execution failed: {}", e)))
            }
        }
    }

    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                permission: t.permission_level(),
                usage: t.usage().to_string(),
            })
            .collect()
    }

    pub fn get_metrics(&self, name: &str) -> Option<&ToolMetrics> {
        self.metrics.get(name)
    }

    pub fn all_metrics(&self) -> &HashMap<String, ToolMetrics> {
        &self.metrics
    }

    pub fn reset_metrics(&mut self) {
        for metrics in self.metrics.values_mut() {
            *metrics = ToolMetrics::new();
        }
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

/// Tool information for listing
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub permission: PermissionLevel,
    pub usage: String,
}

impl std::fmt::Display for ToolInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.description)?;
        if !self.usage.is_empty() {
            write!(f, "\n  Usage: {}", self.usage)?;
        }
        match self.permission {
            PermissionLevel::RequiresConfirmation => write!(f, " [requires confirmation]")?,
            PermissionLevel::Disabled => write!(f, " [disabled]")?,
            _ => {}
        }
        Ok(())
    }
}
