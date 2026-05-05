use crate::client::McpTransport;
use nxc_tools::ToolResult;
use anyhow::Result;
use nxc_config::McpServer as McpServerConfig;
use serde_json::Value;
use tokio::process::{Child, Command};
use std::process::Stdio;

pub struct McpServerHandle {
    _child:    Child,
    transport: McpTransport,
    tools:     Vec<McpToolProxy>,
}

#[derive(Clone)]
pub struct McpToolProxy {
    pub tool_name:   String,
    pub description: String,
    pub parameters:  Value,
}

impl McpServerHandle {
    pub async fn spawn(cfg: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::null());
        for (k, v) in &cfg.env { cmd.env(k, v); }
        let mut child = cmd.spawn()?;

        let stdin  = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut transport = McpTransport::new(stdin, stdout);

        transport.request("initialize", serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "nxc", "version": "0.1.0" }
        })).await?;

        let list = transport.request("tools/list", serde_json::json!({})).await?;
        let tools: Vec<McpToolProxy> = list["tools"].as_array().unwrap_or(&vec![]).iter().map(|t| {
            McpToolProxy {
                tool_name:   t["name"].as_str().unwrap_or("").to_string(),
                description: t["description"].as_str().unwrap_or("").to_string(),
                parameters:  t["inputSchema"].clone(),
            }
        }).collect();

        Ok(Self { _child: child, transport, tools })
    }

    pub fn tool_names(&self) -> Vec<String> { self.tools.iter().map(|t| t.tool_name.clone()).collect() }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<ToolResult> {
        let result = self.transport.request(
            "tools/call",
            serde_json::json!({"name": name, "arguments": args})
        ).await?;
        let content  = result["content"][0]["text"].as_str().unwrap_or("").to_string();
        let is_error = result["isError"].as_bool().unwrap_or(false);
        Ok(if is_error { ToolResult::err(content) } else { ToolResult::ok(content) })
    }
}
