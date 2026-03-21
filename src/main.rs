use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

mod agent;
mod config;
mod error;
mod git;
mod llm;
mod tools;

/// Self-Smart: An AI coding agent powered by local LLM
#[derive(Parser, Debug)]
#[command(name = "self-smart", version, about)]
struct Cli {
    /// vLLM API endpoint
    #[arg(long, default_value = "http://localhost:48000")]
    endpoint: String,

    /// Model ID to use
    #[arg(long, default_value = "Intel/Qwen3.5-9B-int4-AutoRound")]
    model: String,

    /// Working directory
    #[arg(long, default_value = ".")]
    workdir: String,

    /// Enable auto-commit mode
    #[arg(long)]
    auto_commit: bool,

    /// Enable streaming output
    #[arg(long)]
    stream: bool,

    /// Use ReAct agent for multi-step reasoning
    #[arg(long)]
    react: bool,

    /// Maximum iterations for ReAct agent
    #[arg(long, default_value = "10")]
    max_iterations: usize,

    /// Verbose output for ReAct agent
    #[arg(long)]
    verbose: bool,

    /// Prompt or task to execute
    #[arg(short, long)]
    prompt: Option<String>,

    /// Interactive mode
    #[arg(short, long)]
    interactive: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let config = config::Config {
        endpoint: cli.endpoint,
        model: cli.model,
        workdir: cli.workdir,
        auto_commit: cli.auto_commit,
    };

    if let Some(prompt) = cli.prompt {
        if cli.react {
            // Use ReAct agent for multi-step reasoning
            let mut react_agent = agent::react::ReActAgent::new(
                &config,
                cli.max_iterations,
                cli.verbose,
            ).await?;
            let result = react_agent.run(&prompt).await?;
            println!("\nFinal answer:\n{}", result);
        } else {
            // Use standard agent
            let mut agent = agent::Agent::new(config).await?;
            if cli.stream {
                agent.run_streaming(&prompt).await?;
            } else {
                agent.run(&prompt).await?;
            }
        }
    } else if cli.interactive {
        let mut agent = agent::Agent::new(config).await?;
        agent.interactive().await?;
    } else {
        println!("Self-Smart agent ready. Use --prompt or --interactive to start.");
        println!("Run with --help for usage information.");
        println!("\nExamples:");
        println!("  self-smart --prompt 'Analyze src/main.rs'");
        println!("  self-smart --react --verbose --prompt 'Fix all bugs in src/'");
        println!("  self-smart --interactive");
    }

    Ok(())
}
