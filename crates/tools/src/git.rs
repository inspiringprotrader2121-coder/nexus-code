use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct GitStatus;
pub struct GitDiff;
pub struct GitLog;
pub struct GitCommit;
pub struct GitBranch;

fn git(args: &[&str]) -> ToolResult {
    match Command::new("git").args(args).output() {
        Err(e) => ToolResult::err(format!("git error: {e}")),
        Ok(o) => {
            let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if o.status.success() { ToolResult::ok(out) } else { ToolResult::err(out) }
        }
    }
}

#[async_trait] impl Tool for GitStatus {
    fn name(&self)        -> &'static str { "git_status" }
    fn description(&self) -> &'static str { "Show git working tree status" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{}}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(git(&["status"])) }
}
#[async_trait] impl Tool for GitDiff {
    fn name(&self)        -> &'static str { "git_diff" }
    fn description(&self) -> &'static str { "Show staged and unstaged diffs" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"path":{"type":"string"}}}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        if let Some(p) = args["path"].as_str() { Ok(git(&["diff", p])) } else { Ok(git(&["diff"])) }
    }
}
#[async_trait] impl Tool for GitLog {
    fn name(&self)        -> &'static str { "git_log" }
    fn description(&self) -> &'static str { "Show recent git commits" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"n":{"type":"integer","default":10}}}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let n = args["n"].as_u64().unwrap_or(10).to_string();
        Ok(git(&["log", "--oneline", &format!("-{n}")]))
    }
}
#[async_trait] impl Tool for GitCommit {
    fn name(&self)        -> &'static str { "git_commit" }
    fn description(&self) -> &'static str { "Stage all changes and create a commit" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let msg = args["message"].as_str().unwrap_or("chore: update");
        let stage = git(&["add", "-A"]);
        if stage.is_error() { return Ok(stage); }
        Ok(git(&["commit", "-m", msg]))
    }
}
#[async_trait] impl Tool for GitBranch {
    fn name(&self)        -> &'static str { "git_branch" }
    fn description(&self) -> &'static str { "List branches or create a new one" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"name":{"type":"string","description":"Branch name to create (omit to list)"}}}) }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        if let Some(name) = args["name"].as_str() { Ok(git(&["checkout", "-b", name])) }
        else { Ok(git(&["branch", "-a"])) }
    }
}
