use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

#[derive(Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id:      u64,
    pub method:  String,
    pub params:  Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self { jsonrpc: "2.0", id, method: method.into(), params }
    }
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub id:     Option<u64>,
    pub result: Option<Value>,
    pub error:  Option<Value>,
}

pub struct McpTransport {
    stdin:   ChildStdin,
    stdout:  BufReader<ChildStdout>,
    next_id: u64,
}

impl McpTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self { stdin, stdout: BufReader::new(stdout), next_id: 1 }
    }

    pub async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id  = self.next_id;
        self.next_id += 1;
        let req = JsonRpcRequest::new(id, method, params);
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        loop {
            let mut buf = String::new();
            self.stdout.read_line(&mut buf).await?;
            if buf.is_empty() { bail!("MCP server closed stdout"); }
            let resp: JsonRpcResponse = serde_json::from_str(buf.trim())?;
            if resp.id == Some(id) {
                if let Some(err) = resp.error { bail!("MCP error: {err}"); }
                return Ok(resp.result.unwrap_or(Value::Null));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_initialize_request() {
        let req = JsonRpcRequest::new(1, "initialize", serde_json::json!({"protocolVersion":"2024-11-05"}));
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("initialize"));
        assert!(s.contains("jsonrpc"));
    }
}
