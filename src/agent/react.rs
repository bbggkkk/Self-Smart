use crate::agent::Agent;
use crate::config::Config;
use crate::llm::{ConversationContext, LlmClient, Message};
use crate::llm::vllm::VllmClient;
use crate::tools::{ToolRegistry, ToolResult};
use crate::tools::analyze::CodeAnalyzer;
use crate::tools::document::DocGenerator;
use crate::tools::generate::CodeGenerator;
use crate::tools::refactor::Refactorer;
use crate::tools::test::TestRunner;
use anyhow::Result;

/// ReAct (Reason-Act-Observe) agent loop
pub struct ReActAgent {
    llm: VllmClient,
    tools: ToolRegistry,
    context: ConversationContext,
    max_iterations: usize,
    current_iteration: usize,
    verbose: bool,
}

/// Result of a single ReAct step
#[derive(Debug)]
pub enum StepResult {
    /// Agent wants to continue reasoning
    Continue(String),
    /// Agent has completed the task
    Complete(String),
    /// Agent executed a tool
    ToolExecuted {
        tool_name: String,
        args: String,
        result: ToolResult,
    },
    /// Error occurred
    Error(String),
}

impl ReActAgent {
    pub async fn new(config: &Config, max_iterations: usize, verbose: bool) -> Result<Self> {
        let llm = VllmClient::new(&config.endpoint, &config.model);

        let mut tools = ToolRegistry::new();
        tools.register(CodeAnalyzer::new());
        tools.register(DocGenerator::new());
        tools.register(CodeGenerator::new());
        tools.register(Refactorer::new());
        tools.register(TestRunner::new());

        let system_prompt = Self::build_react_prompt(&tools);
        let context = ConversationContext::new(128_000).with_system_prompt(system_prompt);

        Ok(Self {
            llm,
            tools,
            context,
            max_iterations,
            current_iteration: 0,
            verbose,
        })
    }

    pub async fn run(&mut self, task: &str) -> Result<String> {
        if self.verbose {
            println!("\n=== ReAct Agent Starting ===");
            println!("Task: {}", task);
            println!("Max iterations: {}", self.max_iterations);
            println!("{}\n", "=".repeat(50));
        }

        self.context.add_user_message(format!(
            "Task: {}\n\nThink step by step. Use tools when needed. When complete, respond with DONE followed by your final answer.",
            task
        ));

        let mut final_answer = String::new();

        loop {
            if self.current_iteration >= self.max_iterations {
                if self.verbose {
                    println!("\n[Max iterations reached]");
                }
                final_answer = "Max iterations reached. Task may be incomplete.".to_string();
                break;
            }

            self.current_iteration += 1;

            if self.verbose {
                println!("\n--- Iteration {} ---", self.current_iteration);
            }

            let step_result = self.step().await?;

            match step_result {
                StepResult::Complete(answer) => {
                    if self.verbose {
                        println!("\n[Task completed]");
                    }
                    final_answer = answer;
                    break;
                }
                StepResult::Continue(reasoning) => {
                    if self.verbose {
                        println!("Reasoning: {}", reasoning);
                    }
                }
                StepResult::ToolExecuted { tool_name, args, result } => {
                    if self.verbose {
                        println!("Tool '{}' executed on '{}'", tool_name, args);
                        if result.success {
                            println!("Result: {}", &result.output[..result.output.len().min(200)]);
                        } else {
                            println!("Error: {}", result.error.as_deref().unwrap_or("Unknown"));
                        }
                    }
                }
                StepResult::Error(error) => {
                    if self.verbose {
                        println!("Error: {}", error);
                    }
                    self.context.add_assistant_message(format!("Error occurred: {}", error));
                }
            }
        }

        if self.verbose {
            println!("\n=== ReAct Agent Finished ===");
            println!("Iterations used: {}/{}", self.current_iteration, self.max_iterations);
            println!("{}\n", "=".repeat(50));
        }

        Ok(final_answer)
    }

    async fn step(&mut self) -> Result<StepResult> {
        let messages = self.context.get_messages();

        let response = self.llm.chat(messages).await?;

        if self.verbose {
            println!("Response: {}", &response[..response.len().min(300)]);
        }

        // Check if task is complete
        if response.contains("DONE") {
            let answer = response
                .split("DONE")
                .nth(1)
                .unwrap_or(&response)
                .trim();
            self.context.add_assistant_message(&response);
            return Ok(StepResult::Complete(answer.to_string()));
        }

        // Parse tool calls
        if let Some(tool_result) = self.parse_and_execute_tool(&response).await? {
            return Ok(tool_result);
        }

        // No tool call, just reasoning
        self.context.add_assistant_message(&response);
        Ok(StepResult::Continue(response))
    }

    async fn parse_and_execute_tool(&mut self, response: &str) -> Result<Option<StepResult>> {
        let lines: Vec<&str> = response.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("TOOL:") {
                let tool_name = line.trim_start_matches("TOOL:").trim();
                let mut args = String::new();

                if i + 1 < lines.len() && lines[i + 1].trim().starts_with("ARGS:") {
                    args = lines[i + 1]
                        .trim()
                        .trim_start_matches("ARGS:")
                        .trim()
                        .to_string();
                }

                // Execute the tool
                let result = self.tools.execute(tool_name, &args).await?;

                // Add tool execution to context
                self.context.add_assistant_message(format!(
                    "I will use the {} tool on {}.",
                    tool_name, args
                ));

                let tool_feedback = if result.success {
                    format!("Tool {} result:\n{}", tool_name, result.output)
                } else {
                    format!(
                        "Tool {} failed: {}",
                        tool_name,
                        result.error.as_deref().unwrap_or("Unknown error")
                    )
                };

                self.context.add_user_message(tool_feedback);

                return Ok(Some(StepResult::ToolExecuted {
                    tool_name: tool_name.to_string(),
                    args,
                    result,
                }));
            }

            i += 1;
        }

        Ok(None)
    }

    pub fn reset(&mut self) {
        self.context.clear();
        self.current_iteration = 0;
    }

    pub fn context_stats(&self) -> (usize, u32, u32, f32) {
        (
            self.context.message_count(),
            self.context.budget.used_tokens,
            self.context.budget.remaining(),
            self.context.budget.usage_percent(),
        )
    }

    fn build_react_prompt(tools: &ToolRegistry) -> String {
        let tools_desc = tools
            .list_tools()
            .iter()
            .map(|(name, desc)| format!("- {}: {}", name, desc))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are Self-Smart, an AI coding agent using the ReAct (Reason-Act-Observe) framework.

Your goal is to solve coding tasks by thinking step by step and using tools when needed.

Available tools:
{}

Instructions:
1. REASON: Think about what you need to do next
2. ACT: If you need to use a tool, format it as:
   TOOL: <tool_name>
   ARGS: <arguments>
3. OBSERVE: After the tool result, analyze it and decide next steps
4. COMPLETE: When the task is done, respond with:
   DONE
   <your final answer>

Important:
- Be systematic and thorough
- Use tools to gather information before making changes
- Verify your work by running tests when appropriate
- If something fails, try a different approach
- Always provide a clear final answer when complete"#,
            tools_desc
        )
    }
}

/// Multi-step task runner
pub struct MultiStepRunner {
    agent: ReActAgent,
    steps: Vec<String>,
    completed_steps: Vec<String>,
}

impl MultiStepRunner {
    pub async fn new(config: &Config, steps: Vec<String>) -> Result<Self> {
        let agent = ReActAgent::new(config, 20, true).await?;
        Ok(Self {
            agent,
            steps,
            completed_steps: Vec::new(),
        })
    }

    pub async fn run_all(&mut self) -> Result<Vec<String>> {
        let mut results = Vec::new();

        for (i, step) in self.steps.clone().iter().enumerate() {
            println!("\n=== Step {}/{} ===", i + 1, self.steps.len());
            println!("Task: {}", step);

            let result = self.agent.run(step).await?;
            results.push(result.clone());
            self.completed_steps.push(step.clone());

            // Reset context for next step
            self.agent.reset();
        }

        Ok(results)
    }

    pub fn completed_count(&self) -> usize {
        self.completed_steps.len()
    }

    pub fn remaining_count(&self) -> usize {
        self.steps.len() - self.completed_steps.len()
    }
}
