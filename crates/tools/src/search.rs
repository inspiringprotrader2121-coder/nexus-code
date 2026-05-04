use crate::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};

pub struct GrepCodebase;
pub struct FindFiles;

#[async_trait]
impl Tool for GrepCodebase {
    fn name(&self)        -> &'static str { "grep_codebase" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}

#[async_trait]
impl Tool for FindFiles {
    fn name(&self)        -> &'static str { "find_files" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}
