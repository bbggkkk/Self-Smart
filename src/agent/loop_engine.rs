use crate::agent::Agent;
use crate::config::Config;
use anyhow::Result;

pub struct LoopEngine {
    agent: Agent,
    max_iterations: Option<u64>,
}

impl LoopEngine {
    pub async fn new(config: Config, max_iterations: Option<u64>) -> Result<Self> {
        let agent = Agent::new(config).await?;
        Ok(Self {
            agent,
            max_iterations,
        })
    }

    pub async fn run_loop(&mut self, initial_prompt: &str) -> Result<()> {
        let mut iteration = 0u64;
        let current_prompt = initial_prompt.to_string();

        loop {
            iteration += 1;

            if let Some(max) = self.max_iterations {
                if iteration > max {
                    println!("Reached maximum iterations ({}), stopping.", max);
                    break;
                }
            }

            println!("\n--- Loop Iteration {} ---", iteration);

            self.agent.run(&current_prompt).await?;

            // Break after one iteration for safety
            // TODO: Implement proper loop continuation logic
            if iteration >= 1 {
                break;
            }
        }

        Ok(())
    }

    pub async fn continuous_improve(&mut self, task: &str) -> Result<()> {
        println!("Starting continuous improvement loop for task:");
        println!("{}", task);
        println!("{}", "=".repeat(60));

        let mut iteration = 0u64;

        loop {
            iteration += 1;
            println!("\n=== Improvement Iteration {} ===", iteration);

            // Run the agent
            self.agent.run(task).await?;

            // Check git status to see if changes were made
            // If no changes, we might be done
            // This is a simplified version

            println!("\nIteration {} complete.", iteration);

            // For safety, break after a few iterations
            if iteration >= 10 {
                println!("Reached safety limit of 10 iterations.");
                break;
            }
        }

        Ok(())
    }
}
