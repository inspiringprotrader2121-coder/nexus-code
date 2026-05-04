use crate::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};

pub struct GitStatus;
pub struct GitDiff;
pub struct GitLog;
pub struct GitCommit;
pub struct GitBranch;

#[async_trait]
impl Tool for GitStatus {
    fn name(&self)        -> &'static str { "git_status" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}

#[async_trait]
impl Tool for GitDiff {
    fn name(&self)        -> &'static str { "git_diff" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}

#[async_trait]
impl Tool for GitLog {
    fn name(&self)        -> &'static str { "git_log" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}

#[async_trait]
impl Tool for GitCommit {
    fn name(&self)        -> &'static str { "git_commit" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}

#[async_trait]
impl Tool for GitBranch {
    fn name(&self)        -> &'static str { "git_branch" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}
