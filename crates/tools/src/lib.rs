use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;

mod files;
mod shell;
mod search;
mod git;
mod web;
mod vision;

pub use files::{ReadFile, WriteFile, ApplyPatch, ListDir, DeleteFile};
pub use shell::Bash;
pub use search::{GrepCodebase, FindFiles};
pub use git::{GitStatus, GitDiff, GitLog, GitCommit, GitBranch};
pub use web::{FetchUrl, WebSearch};
pub use vision::ReadImage;

pub struct ToolResult {
    pub content:  String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(content: impl Into<String>)  -> Self { Self { content: content.into(), is_error: false } }
    pub fn err(content: impl Into<String>) -> Self { Self { content: content.into(), is_error: true  } }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self)        -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self)  -> Value;
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ReadFile),   Box::new(WriteFile), Box::new(ApplyPatch),
        Box::new(ListDir),    Box::new(DeleteFile),
        Box::new(Bash),
        Box::new(GrepCodebase), Box::new(FindFiles),
        Box::new(GitStatus),  Box::new(GitDiff),   Box::new(GitLog),
        Box::new(GitCommit),  Box::new(GitBranch),
        Box::new(FetchUrl),   Box::new(WebSearch),
        Box::new(ReadImage),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self)        -> &'static str { "echo" }
        fn description(&self) -> &'static str { "echoes input" }
        fn parameters(&self)  -> serde_json::Value {
            serde_json::json!({"type":"object","properties":{"msg":{"type":"string"}},"required":["msg"]})
        }
        async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult::ok(args["msg"].as_str().unwrap_or("").to_string()))
        }
    }

    #[tokio::test]
    async fn tool_executes_and_returns_result() {
        let t   = EchoTool;
        let res = t.execute(serde_json::json!({"msg": "hello"})).await.unwrap();
        assert!(!res.is_error);
        assert_eq!(res.content, "hello");
    }

    #[test]
    fn all_tools_have_unique_names() {
        let tools = all_tools();
        let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name()).collect();
        assert_eq!(names.len(), tools.len());
    }
}
