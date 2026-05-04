use crate::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};

pub struct ReadImage;

#[async_trait]
impl Tool for ReadImage {
    fn name(&self)        -> &'static str { "read_image" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}
