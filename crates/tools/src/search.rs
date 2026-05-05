use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use ignore::WalkBuilder;
use serde_json::{json, Value};

pub struct GrepCodebase;
pub struct FindFiles;

#[async_trait]
impl Tool for GrepCodebase {
    fn name(&self)        -> &'static str { "grep_codebase" }
    fn description(&self) -> &'static str { "Search for a text pattern across files, respecting .gitignore" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string","description":"Directory to search (default: cwd)"}},"required":["pattern"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let root    = args["path"].as_str().unwrap_or(".");
        let mut matches: Vec<String> = vec![];
        for entry in WalkBuilder::new(root).build().filter_map(|e| e.ok()) {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(entry.path()) {
                    for (i, line) in text.lines().enumerate() {
                        if line.contains(pattern) {
                            matches.push(format!("{}:{}: {}", entry.path().display(), i + 1, line));
                        }
                    }
                }
            }
        }
        if matches.is_empty() { Ok(ToolResult::ok("No matches found.")) }
        else { Ok(ToolResult::ok(matches.join("\n"))) }
    }
}

#[async_trait]
impl Tool for FindFiles {
    fn name(&self)        -> &'static str { "find_files" }
    fn description(&self) -> &'static str { "Find files matching a glob pattern, respecting .gitignore" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"glob":{"type":"string"}},"required":["glob"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let pattern = args["glob"].as_str().unwrap_or("*");
        let matcher = glob::Pattern::new(pattern).unwrap_or_else(|_| glob::Pattern::new("*").unwrap());
        let mut found: Vec<String> = vec![];
        for entry in WalkBuilder::new(".").build().filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy();
            if matcher.matches(&name) { found.push(entry.path().display().to_string()); }
        }
        found.sort();
        if found.is_empty() { Ok(ToolResult::ok("No files found.")) }
        else { Ok(ToolResult::ok(found.join("\n"))) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    #[tokio::test]
    async fn grep_finds_pattern_in_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "fn hello() {}").unwrap();
        fs::write(dir.path().join("b.rs"), "fn world() {}").unwrap();
        let res = GrepCodebase.execute(serde_json::json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap()
        })).await.unwrap();
        assert!(res.content.contains("hello"));
        assert!(!res.content.contains("world"));
    }
}
