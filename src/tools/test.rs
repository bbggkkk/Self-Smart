use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Command;

pub struct TestRunner;

impl TestRunner {
    pub fn new() -> Self {
        Self
    }

    fn run_cargo_test(&self, dir: &Path) -> Result<String> {
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(dir)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(format!("Tests passed!\n{}", stdout))
        } else {
            Ok(format!("Tests failed:\n{}\n{}", stdout, stderr))
        }
    }

    fn run_cargo_check(&self, dir: &Path) -> Result<String> {
        let output = Command::new("cargo")
            .arg("check")
            .current_dir(dir)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(format!("Check passed!\n{}", stdout))
        } else {
            Ok(format!("Check failed:\n{}\n{}", stdout, stderr))
        }
    }

    fn run_cargo_clippy(&self, dir: &Path) -> Result<String> {
        let output = Command::new("cargo")
            .args(["clippy", "--", "-D", "warnings"])
            .current_dir(dir)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(format!("Clippy passed!\n{}", stdout))
        } else {
            Ok(format!("Clippy warnings/errors:\n{}\n{}", stdout, stderr))
        }
    }
}

#[async_trait]
impl Tool for TestRunner {
    fn name(&self) -> &str {
        "test"
    }

    fn description(&self) -> &str {
        "Run tests, checks, and linting (cargo test, cargo check, cargo clippy)"
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let command = parts.first().copied().unwrap_or("test");
        let dir = parts.get(1).copied().unwrap_or(".");

        let path = Path::new(dir);
        if !path.exists() {
            return Ok(ToolResult::error(format!("Directory not found: {}", dir)));
        }

        let result = match command {
            "test" => self.run_cargo_test(path)?,
            "check" => self.run_cargo_check(path)?,
            "clippy" => self.run_cargo_clippy(path)?,
            _ => {
                return Ok(ToolResult::error(format!(
                    "Unknown test command: {}. Use 'test', 'check', or 'clippy'",
                    command
                )))
            }
        };

        Ok(ToolResult::success(result))
    }
}
