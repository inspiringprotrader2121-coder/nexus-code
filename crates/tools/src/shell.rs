use crate::{Tool, ToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct Bash;

#[async_trait]
impl Tool for Bash {
    fn name(&self)        -> &'static str { "bash" }
    fn description(&self) -> &'static str { "Run a shell command. stdout and stderr are returned." }
    fn parameters(&self)  -> Value {
        json!({"type":"object","properties":{"command":{"type":"string","description":"Shell command to run"}},"required":["command"]})
    }
    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let cmd = args["command"].as_str().unwrap_or("");
        let out = Command::new("sh").arg("-c").arg(cmd).output()?;
        let mut content = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if !stderr.is_empty() { content.push_str(&format!("\nstderr:\n{stderr}")); }
        if out.status.success() { Ok(ToolResult::ok(content)) }
        else { Ok(ToolResult::err(format!("exit {}\n{content}", out.status.code().unwrap_or(-1)))) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn bash_runs_command_and_captures_output() {
        let res = Bash.execute(serde_json::json!({"command": "echo hello"})).await.unwrap();
        assert!(!res.is_error);
        assert!(res.content.contains("hello"));
    }
}
