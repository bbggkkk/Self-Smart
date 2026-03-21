pub mod loop_engine;

use crate::config::Config;
use crate::git::GitManager;
use crate::llm::{ConversationContext, LlmClient, Message};
use crate::llm::vllm::VllmClient;
use crate::tools::ToolRegistry;
use crate::tools::analyze::CodeAnalyzer;
use crate::tools::document::DocGenerator;
use crate::tools::generate::CodeGenerator;
use crate::tools::refactor::Refactorer;
use crate::tools::test::TestRunner;
use anyhow::Result;
use std::io::{self, Write};

pub struct Agent {
    config: Config,
    llm: VllmClient,
    tools: ToolRegistry,
    git: Option<GitManager>,
    context: ConversationContext,
    iteration: u64,
    total_tokens_used: u64,
}

impl Agent {
    pub async fn new(config: Config) -> Result<Self> {
        let llm = VllmClient::new(&config.endpoint, &config.model);

        let mut tools = ToolRegistry::new();
        tools.register(CodeAnalyzer::new());
        tools.register(DocGenerator::new());
        tools.register(CodeGenerator::new());
        tools.register(Refactorer::new());
        tools.register(TestRunner::new());

        let git = GitManager::new(&config.workdir).ok();

        let system_prompt = Self::build_system_prompt(&tools);
        let context = ConversationContext::new(128_000).with_system_prompt(system_prompt);

        Ok(Self {
            config,
            llm,
            tools,
            git,
            context,
            iteration: 0,
            total_tokens_used: 0,
        })
    }

    pub async fn run(&mut self, prompt: &str) -> Result<()> {
        println!("Self-Smart Agent - Iteration {}", self.iteration + 1);
        println!("Task: {}", prompt);
        println!(
            "Context: {} messages, {:.1}% budget used",
            self.context.message_count(),
            self.context.budget.usage_percent()
        );
        println!("{}", "=".repeat(60));

        self.context.add_user_message(prompt);

        // Trim context if needed before sending
        self.context.trim_to_budget();

        let messages = self.context.get_messages();

        println!("\nThinking...\n");

        let (response, usage) = self.llm.chat_with_usage(messages).await?;

        println!("{}", response);
        println!("\n{}", "=".repeat(60));

        // Track token usage
        if let Some(usage) = &usage {
            self.total_tokens_used += usage.total_tokens as u64;
            println!(
                "Tokens: {} prompt + {} completion = {} total (session: {})",
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens,
                self.total_tokens_used
            );
        }

        self.context.add_assistant_message(&response);

        // Parse and execute tools if needed
        self.execute_tool_calls(&response).await?;

        // Auto-commit if enabled
        if self.config.auto_commit {
            self.auto_commit().await?;
        }

        self.iteration += 1;

        Ok(())
    }

    pub async fn run_streaming(&mut self, prompt: &str) -> Result<()> {
        println!("Self-Smart Agent - Iteration {} (Streaming)", self.iteration + 1);
        println!("Task: {}", prompt);
        println!("{}", "=".repeat(60));

        self.context.add_user_message(prompt);
        self.context.trim_to_budget();
        let messages = self.context.get_messages();

        println!("\nThinking...\n");

        let response = self.llm.chat_stream(messages, |chunk| {
            print!("{}", chunk);
            io::stdout().flush().unwrap_or(());
        }).await?;

        println!("\n{}", "=".repeat(60));

        self.context.add_assistant_message(&response);
        self.execute_tool_calls(&response).await?;

        if self.config.auto_commit {
            self.auto_commit().await?;
        }

        self.iteration += 1;

        Ok(())
    }

    pub async fn interactive(&mut self) -> Result<()> {
        println!("Self-Smart Agent - Interactive Mode");
        println!("Type 'quit' or 'exit' to stop");
        println!("Type 'tools' to list available tools");
        println!("Type 'history' to see conversation history");
        println!("Type 'context' to see context stats");
        println!("Type 'clear' to clear conversation history");
        println!("Type 'status' to see git status");
        println!("{}", "=".repeat(60));

        loop {
            print!("\n> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match input {
                "quit" | "exit" => {
                    println!("Goodbye!");
                    println!("Session stats:");
                    println!("  Iterations: {}", self.iteration);
                    println!("  Total tokens: {}", self.total_tokens_used);
                    break;
                }
                "tools" => {
                    println!("\nAvailable tools:");
                    for (name, desc) in self.tools.list_tools() {
                        println!("  {} - {}", name, desc);
                    }
                    continue;
                }
                "history" => {
                    println!("\nConversation history ({} messages):", self.context.message_count());
                    for msg in &self.context.messages {
                        let preview = if msg.content.len() > 100 {
                            format!("{}...", &msg.content[..100])
                        } else {
                            msg.content.clone()
                        };
                        println!("  [{}]: {}", msg.role, preview);
                    }
                    continue;
                }
                "context" => {
                    println!("\nContext stats:");
                    println!("  Messages: {}", self.context.message_count());
                    println!(
                        "  Budget: {}/{} tokens ({:.1}% used)",
                        self.context.budget.used_tokens,
                        self.context.budget.max_tokens,
                        self.context.budget.usage_percent()
                    );
                    println!("  Remaining: {} tokens", self.context.budget.remaining());
                    println!("  Session total: {} tokens", self.total_tokens_used);
                    continue;
                }
                "clear" => {
                    self.context.clear();
                    println!("Conversation cleared.");
                    continue;
                }
                "status" => {
                    if let Some(git) = &self.git {
                        println!("\nGit status:");
                        println!("{}", git.status()?);
                    } else {
                        println!("Not in a git repository");
                    }
                    continue;
                }
                _ => {}
            }

            self.run(input).await?;
        }

        Ok(())
    }

    pub fn clear_context(&mut self) {
        self.context.clear();
    }

    pub fn context_stats(&self) -> (usize, u32, u32, f32) {
        (
            self.context.message_count(),
            self.context.budget.used_tokens,
            self.context.budget.remaining(),
            self.context.budget.usage_percent(),
        )
    }

    fn build_system_prompt(tools: &ToolRegistry) -> String {
        let tools_desc = tools
            .list_tools()
            .iter()
            .map(|(name, desc)| format!("- {}: {}", name, desc))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are Self-Smart, an AI coding agent powered by local LLM. You help with coding tasks including:
- Code generation and editing
- Code analysis and understanding
- Debugging and fixing issues
- Refactoring and improving code
- Running tests
- Documentation

Available tools:
{}

When you want to use a tool, format your request as:
TOOL: <tool_name>
ARGS: <arguments>

For example:
TOOL: analyze
ARGS: src/main.rs

Be concise and helpful. Focus on solving the user's problem efficiently."#,
            tools_desc
        )
    }

    async fn execute_tool_calls(&mut self, response: &str) -> Result<()> {
        let lines: Vec<&str> = response.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("TOOL:") {
                let tool_name = lines[i].trim_start_matches("TOOL:").trim();
                let mut args = String::new();

                if i + 1 < lines.len() && lines[i + 1].starts_with("ARGS:") {
                    args = lines[i + 1].trim_start_matches("ARGS:").trim().to_string();
                    i += 2;
                } else {
                    i += 1;
                }

                println!("\nExecuting tool: {} with args: {}", tool_name, args);
                let result = self.tools.execute(tool_name, &args).await?;

                if result.success {
                    println!("Result:\n{}", result.output);
                    // Add tool result to context
                    self.context.add_assistant_message(format!(
                        "Tool {} executed successfully:\n{}",
                        tool_name, result.output
                    ));
                } else {
                    let error = result.error.unwrap_or_default();
                    println!("Error: {}", error);
                    self.context.add_assistant_message(format!(
                        "Tool {} failed: {}",
                        tool_name, error
                    ));
                }
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    async fn auto_commit(&mut self) -> Result<()> {
        if let Some(git) = &self.git {
            let status = git.status()?;
            if !status.trim().is_empty() {
                println!("\nAuto-committing changes...");
                git.add_all()?;

                let message = format!("Iteration {}: {}", self.iteration + 1, "Auto-commit");
                git.commit(&message)?;

                let tag_name = format!("v0.1.{}", self.iteration + 1);
                git.tag(
                    &tag_name,
                    &format!(
                        "Iteration {} completed | Tokens: {}",
                        self.iteration + 1,
                        self.total_tokens_used
                    ),
                )?;

                println!("Committed and tagged as {}", tag_name);
            }
        }
        Ok(())
    }
}
