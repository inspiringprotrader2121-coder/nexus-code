use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct FetchUrl;
pub struct WebSearch;

#[async_trait]
impl Tool for FetchUrl {
    fn name(&self)        -> &'static str { "fetch_url" }
    fn description(&self) -> &'static str { "Fetch a URL and return its text content" }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let url = args["url"].as_str().unwrap_or("");
        match reqwest::get(url).await {
            Err(e)   => Ok(ToolResult::err(format!("fetch error: {e}"))),
            Ok(resp) => match resp.text().await {
                Ok(text) => Ok(ToolResult::ok(text.chars().take(20_000).collect::<String>())),
                Err(e)   => Ok(ToolResult::err(format!("read error: {e}"))),
            },
        }
    }
}

#[async_trait]
impl Tool for WebSearch {
    fn name(&self)        -> &'static str { "web_search" }
    fn description(&self) -> &'static str { "Search the web for a query. Returns top results." }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let query = args["query"].as_str().unwrap_or("");
        let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query));
        match reqwest::get(&url).await {
            Err(e)   => Ok(ToolResult::err(format!("search error: {e}"))),
            Ok(resp) => match resp.text().await {
                Ok(text) => Ok(ToolResult::ok(text.chars().take(8_000).collect::<String>())),
                Err(e)   => Ok(ToolResult::err(format!("search read error: {e}"))),
            },
        }
    }
}
