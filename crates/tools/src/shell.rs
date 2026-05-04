use crate::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};

pub struct Bash;

#[async_trait]
impl Tool for Bash {
    fn name(&self)        -> &'static str { "bash" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}
