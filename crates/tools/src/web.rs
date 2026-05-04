use crate::{Tool, ToolResult};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};

pub struct FetchUrl;
pub struct WebSearch;

#[async_trait]
impl Tool for FetchUrl {
    fn name(&self)        -> &'static str { "fetch_url" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}

#[async_trait]
impl Tool for WebSearch {
    fn name(&self)        -> &'static str { "web_search" }
    fn description(&self) -> &'static str { "stub" }
    fn parameters(&self)  -> Value { json!({}) }
    async fn execute(&self, _: Value) -> Result<ToolResult> { Ok(ToolResult::ok("stub")) }
}
