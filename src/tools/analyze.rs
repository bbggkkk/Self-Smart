use super::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use walkdir::WalkDir;

pub struct CodeAnalyzer;

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn analyze_file(&self, path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");

        let mut analysis = Vec::new();
        analysis.push(format!("File: {}", path.display()));
        analysis.push(format!("Language: {}", extension));
        analysis.push(format!("Lines: {}", content.lines().count()));
        analysis.push(format!("Size: {} bytes", content.len()));

        // Basic code metrics
        let functions = content.matches("fn ").count();
        let structs = content.matches("struct ").count();
        let enums = content.matches("enum ").count();
        let impls = content.matches("impl ").count();

        if functions > 0 {
            analysis.push(format!("Functions: {}", functions));
        }
        if structs > 0 {
            analysis.push(format!("Structs: {}", structs));
        }
        if enums > 0 {
            analysis.push(format!("Enums: {}", enums));
        }
        if impls > 0 {
            analysis.push(format!("Impl blocks: {}", impls));
        }

        // Check for TODOs and FIXMEs
        let todos: Vec<_> = content
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains("TODO") || line.contains("FIXME"))
            .map(|(i, line)| format!("  Line {}: {}", i + 1, line.trim()))
            .collect();

        if !todos.is_empty() {
            analysis.push("TODOs/FIXMEs:".to_string());
            analysis.extend(todos);
        }

        Ok(analysis.join("\n"))
    }

    fn analyze_directory(&self, dir: &Path) -> Result<String> {
        let mut results = Vec::new();
        let mut total_lines = 0;
        let mut total_files = 0;

        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp") {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            total_lines += content.lines().count();
                            total_files += 1;
                        }
                    }
                }
            }
        }

        results.push(format!("Directory: {}", dir.display()));
        results.push(format!("Code files: {}", total_files));
        results.push(format!("Total lines: {}", total_lines));

        Ok(results.join("\n"))
    }
}

#[async_trait]
impl Tool for CodeAnalyzer {
    fn name(&self) -> &str {
        "analyze"
    }

    fn description(&self) -> &str {
        "Analyze code files or directories for metrics and structure"
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let path = Path::new(args.trim());

        if !path.exists() {
            return Ok(ToolResult::error(format!("Path not found: {}", path.display())));
        }

        let result = if path.is_file() {
            self.analyze_file(path)
        } else {
            self.analyze_directory(path)
        }?;

        Ok(ToolResult::success(result))
    }
}
