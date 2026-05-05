use crate::{approval::{needs_approval, prompt_approval}, history::History};
use anyhow::Result;
use nxc_config::Config;
use nxc_provider::{Client, Message, ToolDef, FunctionDef};
use nxc_tools::Tool;
use serde_json::Value;

pub struct Agent {
    pub history: History,
    client:      Client,
    tools:       Vec<Box<dyn Tool>>,
    config:      Config,
}

pub struct TurnCallbacks<'a> {
    pub on_text:   &'a dyn Fn(&str),
    pub on_action: &'a dyn Fn(&str, &str),
}

impl Agent {
    pub fn new(config: Config, tools: Vec<Box<dyn Tool>>) -> Self {
        let client = Client::new(
            config.provider.base_url.clone(),
            config.provider.api_key.clone(),
            config.provider.model.clone(),
        );
        let context_limit = config.agent.context_limit;
        Self { history: History::new(context_limit), client, tools, config }
    }

    pub fn tool_defs(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| ToolDef {
            kind: "function",
            function: FunctionDef { name: t.name(), description: t.description(), parameters: t.parameters() },
        }).collect()
    }

    pub async fn run_turn(&mut self, callbacks: TurnCallbacks<'_>) -> Result<bool> {
        self.history.truncate_to_limit();
        let defs = self.tool_defs();
        let resp = self.client.complete(&self.history.messages, &defs, callbacks.on_text).await?;

        if resp.tool_calls.is_empty() {
            if !resp.text.is_empty() {
                self.history.push(Message::assistant_text(&resp.text));
            }
            return Ok(false);
        }

        self.history.push(Message::assistant_tool_calls(resp.tool_calls.clone()));

        for call in &resp.tool_calls {
            let args: Value = serde_json::from_str(&call.function.arguments).unwrap_or(Value::Null);
            let preview = call.function.arguments.clone();

            let approved = if needs_approval(&call.function.name, &self.config.agent.approval_mode, &self.config.tools) {
                (callbacks.on_action)(&call.function.name, &preview);
                prompt_approval(&call.function.name, &preview)
            } else {
                (callbacks.on_action)(&call.function.name, &preview);
                true
            };

            let result = if approved {
                let tool = self.tools.iter().find(|t| t.name() == call.function.name);
                match tool {
                    Some(t) => t.execute(args).await?,
                    None    => nxc_tools::ToolResult::err(format!("Unknown tool: {}", call.function.name)),
                }
            } else {
                nxc_tools::ToolResult::err("User denied tool execution")
            };

            self.history.push(Message::tool_result(&call.id, result.content));
        }

        Ok(true)
    }
}
