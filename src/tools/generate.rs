use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub path: String,
    pub description: String,
    pub language: Option<String>,
}

pub struct CodeGenerator;

impl CodeGenerator {
    pub fn new() -> Self {
        Self
    }

    fn detect_language(&self, path: &Path) -> &str {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("js") => "javascript",
            Some("ts") => "typescript",
            Some("go") => "go",
            Some("c") => "c",
            Some("cpp") => "cpp",
            _ => "unknown",
        }
    }

    fn generate_boilerplate(&self, path: &Path, language: &str) -> Result<String> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module");

        let content = match language {
            "rust" => format!(
                r#"//! {name} module

use anyhow::Result;

/// TODO: Implement {name} functionality
pub fn run() -> Result<()> {{
    todo!("Implement {name}")
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_run() {{
        // TODO: Add tests
    }}
}}
"#
            ),
            "python" => format!(
                r#""""{name} module."""


def run():
    """TODO: Implement {name} functionality."""
    raise NotImplementedError("Implement {name}")


if __name__ == "__main__":
    run()
"#
            ),
            _ => format!("// {} module\n// TODO: Implement functionality\n", name),
        };

        Ok(content)
    }
}

#[async_trait]
impl Tool for CodeGenerator {
    fn name(&self) -> &str {
        "generate"
    }

    fn description(&self) -> &str {
        "Generate code files with boilerplate or from description"
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let request: GenerateRequest = match serde_json::from_str(args) {
            Ok(req) => req,
            Err(_) => {
                // Treat args as a file path
                GenerateRequest {
                    path: args.trim().to_string(),
                    description: "Generate boilerplate".to_string(),
                    language: None,
                }
            }
        };

        let path = Path::new(&request.path);
        let language = request
            .language
            .as_deref()
            .unwrap_or_else(|| self.detect_language(path));

        if path.exists() {
            return Ok(ToolResult::error(format!(
                "File already exists: {}",
                path.display()
            )));
        }

        let content = self.generate_boilerplate(path, language)?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, &content)?;

        Ok(ToolResult::success(format!(
            "Created {} with {} boilerplate",
            path.display(),
            language
        )))
    }
}
