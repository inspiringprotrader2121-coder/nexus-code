use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: None, name: None }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: None, name: None }
    }
    pub fn assistant_text(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: None, name: None }
    }
    pub fn assistant_tool_calls(calls: Vec<ToolCall>) -> Self {
        Self { role: "assistant".into(), content: None, tool_calls: Some(calls), tool_call_id: None, name: None }
    }
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: "tool".into(), content: Some(Value::String(content.into())), tool_calls: None, tool_call_id: Some(tool_call_id.into()), name: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

#[derive(Debug, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug)]
pub struct CompletionResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
