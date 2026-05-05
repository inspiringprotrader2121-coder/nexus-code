use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_model()    -> String { "anthropic/claude-sonnet-4-6".into() }
fn default_base_url() -> String { "https://openrouter.ai/api/v1".into() }

impl Default for ProviderConfig {
    fn default() -> Self {
        Self { api_key: String::new(), model: default_model(), base_url: default_base_url() }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalMode {
    #[default]
    Auto,
    Ask,
    Yolo,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    #[serde(default = "ApprovalMode::default")]
    pub approval_mode: ApprovalMode,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default = "default_context_limit")]
    pub context_limit: u32,
}

fn default_max_turns()     -> u32 { 50 }
fn default_context_limit() -> u32 { 128_000 }

impl Default for AgentConfig {
    fn default() -> Self {
        Self { approval_mode: ApprovalMode::Auto, max_turns: 50, context_limit: 128_000 }
    }
}


#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub approval: HashMap<String, ApprovalMode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServer>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SearchConfig {
    pub serpapi_key: Option<String>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_approval_mode_is_auto() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.approval_mode, ApprovalMode::Auto);
        assert_eq!(cfg.max_turns, 50);
        assert_eq!(cfg.context_limit, 128_000);
    }

    #[test]
    fn approval_mode_deserializes_from_string() {
        let toml = r#"approval_mode = "yolo""#;
        let cfg: AgentConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.approval_mode, ApprovalMode::Yolo);
    }
}
