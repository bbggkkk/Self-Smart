use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct RefactorRequest {
    pub path: String,
    pub operation: String,
    pub target: Option<String>,
    pub replacement: Option<String>,
}

pub struct Refactorer;

impl Refactorer {
    pub fn new() -> Self {
        Self
    }

    fn detect_code_smells(&self, content: &str) -> Vec<String> {
        let mut smells = Vec::new();

        // Check for long functions (>50 lines)
        let mut fn_start = None;
        let mut brace_count = 0;
        for (i, line) in content.lines().enumerate() {
            if line.contains("fn ") && line.contains('{') {
                fn_start = Some(i);
                brace_count = 1;
            } else if let Some(start) = fn_start {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;
                if brace_count == 0 {
                    let length = i - start;
                    if length > 50 {
                        smells.push(format!(
                            "Long function starting at line {} ({} lines)",
                            start + 1,
                            length
                        ));
                    }
                    fn_start = None;
                }
            }
        }

        // Check for magic numbers
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("//")
                && !trimmed.starts_with("*")
                && trimmed.chars().any(|c| c.is_ascii_digit())
            {
                for word in trimmed.split_whitespace() {
                    if let Ok(num) = word.parse::<i64>() {
                        if num != 0 && num != 1 && num != -1 && !word.contains('.') {
                            smells.push(format!(
                                "Possible magic number {} at line {}",
                                num,
                                i + 1
                            ));
                        }
                    }
                }
            }
        }

        // Check for duplicate code patterns
        let lines: Vec<&str> = content.lines().collect();
        for i in 0..lines.len().saturating_sub(5) {
            for j in (i + 5)..lines.len().saturating_sub(5) {
                let window1: String = lines[i..i + 5].join("\n");
                let window2: String = lines[j..j + 5].join("\n");
                if window1 == window2 && !window1.trim().is_empty() {
                    smells.push(format!("Duplicate code block at lines {} and {}", i + 1, j + 1));
                }
            }
        }

        smells
    }
}

#[async_trait]
impl Tool for Refactorer {
    fn name(&self) -> &str {
        "refactor"
    }

    fn description(&self) -> &str {
        "Analyze code for refactoring opportunities and apply refactoring"
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let request: RefactorRequest = match serde_json::from_str(args) {
            Ok(req) => req,
            Err(_) => RefactorRequest {
                path: args.trim().to_string(),
                operation: "analyze".to_string(),
                target: None,
                replacement: None,
            },
        };

        let path = Path::new(&request.path);

        if !path.exists() {
            return Ok(ToolResult::error(format!("Path not found: {}", path.display())));
        }

        let content = std::fs::read_to_string(path)?;

        match request.operation.as_str() {
            "analyze" => {
                let smells = self.detect_code_smells(&content);
                if smells.is_empty() {
                    Ok(ToolResult::success("No obvious code smells detected.".to_string()))
                } else {
                    Ok(ToolResult::success(format!(
                        "Found {} potential issues:\n{}",
                        smells.len(),
                        smells.join("\n")
                    )))
                }
            }
            "rename" => {
                let target = request
                    .target
                    .ok_or_else(|| anyhow::anyhow!("Target name required for rename"))?;
                let replacement = request
                    .replacement
                    .ok_or_else(|| anyhow::anyhow!("Replacement name required for rename"))?;

                let new_content = content.replace(&target, &replacement);
                std::fs::write(path, &new_content)?;

                Ok(ToolResult::success(format!(
                    "Renamed '{}' to '{}' in {}",
                    target,
                    replacement,
                    path.display()
                )))
            }
            _ => Ok(ToolResult::error(format!(
                "Unknown operation: {}",
                request.operation
            ))),
        }
    }
}
