pub mod loop_engine;

use crate::config::Config;
use crate::git::GitManager;
use crate::llm::{LlmClient, Message};
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
    history: Vec<Message>,
    iteration: u64,
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

        Ok(Self {
            config,
            llm,
            tools,
            git,
            history: Vec::new(),
            iteration: 0,
        })
    }

    pub async fn run(&mut self, prompt: &str) -> Result<()> {
        println!("Self-Smart Agent - Iteration {}", self.iteration + 1);
        println!("Task: {}", prompt);
        println!("{}", "=".repeat(60));

        self.history.push(Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let system_prompt = self.build_system_prompt();
        let messages = self.build_messages(&system_prompt);

        println!("\nThinking...\n");

        let response = self.llm.chat(messages).await?;

        println!("{}", response);
        println!("\n{}", "=".repeat(60));

        self.history.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
        });

        // Parse and execute tools if needed
        self.execute_tool_calls(&response).await?;

        // Auto-commit if enabled
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
                    println!("\nConversation history:");
                    for msg in &self.history {
                        println!("  [{}]: {}", msg.role, &msg.content[..msg.content.len().min(100)]);
                    }
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

    fn build_system_prompt(&self) -> String {
        let tools_desc = self
            .tools
            .list_tools()
            .iter()
            .map(|(name, desc)| format!("- {}: {}", name, desc))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are Self-Smart, an AI coding agent. You help with coding tasks including:
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

    fn build_messages(&self, system_prompt: &str) -> Vec<Message> {
        let mut messages = vec![Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        }];
        messages.extend(self.history.clone());
        messages
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
                } else {
                    println!("Error: {}", result.error.unwrap_or_default());
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
                git.tag(&tag_name, &format!("Iteration {} completed", self.iteration + 1))?;

                println!("Committed and tagged as {}", tag_name);
            }
        }
        Ok(())
    }
}
