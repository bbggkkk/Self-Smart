use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

pub struct DocGenerator;

impl DocGenerator {
    pub fn new() -> Self {
        Self
    }

    fn generate_file_docs(&self, path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");

        let mut docs = Vec::new();
        docs.push(format!("# Documentation for {}", path.display()));
        docs.push(String::new());

        match extension {
            "rs" => {
                docs.push(self.extract_rust_docs(&content));
            }
            "py" => {
                docs.push(self.extract_python_docs(&content));
            }
            _ => {
                docs.push(format!("File type '{}' - basic documentation:", extension));
                docs.push(format!("- Lines: {}", content.lines().count()));
                docs.push(format!("- Size: {} bytes", content.len()));
            }
        }

        Ok(docs.join("\n"))
    }

    fn extract_rust_docs(&self, content: &str) -> String {
        let mut docs = Vec::new();

        // Extract public functions
        for line in content.lines() {
            if line.contains("pub fn ") {
                if let Some(name) = line.split("pub fn ").nth(1) {
                    let name = name.split('(').next().unwrap_or(name).trim();
                    docs.push(format!("## Function: `{}`", name));
                    docs.push(format!("```rust\n{}\n```\n", line.trim()));
                }
            }
        }

        // Extract structs
        for line in content.lines() {
            if line.contains("pub struct ") {
                if let Some(name) = line.split("pub struct ").nth(1) {
                    let name = name.split('{').next().unwrap_or(name).trim();
                    docs.push(format!("## Struct: `{}`", name));
                }
            }
        }

        if docs.is_empty() {
            docs.push("No public API documentation found.".to_string());
        }

        docs.join("\n")
    }

    fn extract_python_docs(&self, content: &str) -> String {
        let mut docs = Vec::new();

        for line in content.lines() {
            if line.starts_with("def ") || line.starts_with("async def ") {
                let name = line
                    .split("def ")
                    .nth(1)
                    .and_then(|s| s.split('(').next())
                    .unwrap_or("unknown");
                docs.push(format!("## Function: `{}`", name.trim()));
            }
        }

        if docs.is_empty() {
            docs.push("No function documentation found.".to_string());
        }

        docs.join("\n")
    }
}

#[async_trait]
impl Tool for DocGenerator {
    fn name(&self) -> &str {
        "document"
    }

    fn description(&self) -> &str {
        "Generate documentation for code files"
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let path = Path::new(args.trim());

        if !path.exists() {
            return Ok(ToolResult::error(format!("Path not found: {}", path.display())));
        }

        let docs = self.generate_file_docs(path)?;
        Ok(ToolResult::success(docs))
    }
}
