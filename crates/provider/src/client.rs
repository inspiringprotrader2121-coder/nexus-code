use crate::types::*;
use anyhow::{bail, Result};
use futures_util::StreamExt;
use serde_json::{json, Value};

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl Client {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    pub async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        mut on_text: impl FnMut(&str),
    ) -> Result<CompletionResponse> {
        let mut body = json!({
            "model":   self.model,
            "messages": messages,
            "stream":  true,
            "stream_options": { "include_usage": true },
        });
        if !tools.is_empty() {
            body["tools"] = json!(tools);
        }

        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("OpenRouter error {status}: {text}");
        }

        let mut stream = resp.bytes_stream();
        let mut full_text = String::new();
        let mut partials: Vec<PartialToolCall> = vec![];
        let mut usage = Usage::default();
        let mut buf = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buf.push_str(&String::from_utf8_lossy(&bytes));

            loop {
                match buf.find('\n') {
                    None => break,
                    Some(nl) => {
                        let line = buf[..nl].trim().to_string();
                        buf = buf[nl + 1..].to_string();

                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                break;
                            }
                            if let Ok(evt) = serde_json::from_str::<Value>(data) {
                                process_chunk(
                                    &evt,
                                    &mut full_text,
                                    &mut partials,
                                    &mut usage,
                                    &mut on_text,
                                );
                            }
                        }
                    }
                }
            }
        }

        let tool_calls = partials
            .into_iter()
            .filter(|p| !p.name.is_empty())
            .map(|p| ToolCall {
                id: p.id,
                kind: "function".into(),
                function: FunctionCall {
                    name: p.name,
                    arguments: p.arguments,
                },
            })
            .collect();

        Ok(CompletionResponse {
            text: full_text,
            tool_calls,
            usage,
        })
    }
}

fn process_chunk(
    evt: &Value,
    text: &mut String,
    tool_calls: &mut Vec<PartialToolCall>,
    usage: &mut Usage,
    on_text: &mut impl FnMut(&str),
) {
    if let Some(u) = evt.get("usage").and_then(|v| v.as_object()) {
        if let Some(p) = u.get("prompt_tokens").and_then(|v| v.as_u64()) {
            usage.prompt_tokens = p as u32;
        }
        if let Some(c) = u.get("completion_tokens").and_then(|v| v.as_u64()) {
            usage.completion_tokens = c as u32;
        }
    }

    let delta = match evt["choices"][0]["delta"].as_object() {
        Some(d) => d,
        None => return,
    };

    if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
        if !content.is_empty() {
            on_text(content);
            text.push_str(content);
        }
    }

    if let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
        for call in calls {
            let idx = call["index"].as_u64().unwrap_or(0) as usize;
            while tool_calls.len() <= idx {
                tool_calls.push(PartialToolCall::default());
            }
            if let Some(id) = call["id"].as_str() {
                tool_calls[idx].id = id.into();
            }
            if let Some(name) = call["function"]["name"].as_str() {
                tool_calls[idx].name = name.into();
            }
            if let Some(args) = call["function"]["arguments"].as_str() {
                tool_calls[idx].arguments.push_str(args);
            }
        }
    }
}

#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn complete_returns_text_response() {
        let server = MockServer::start().await;
        let sse_body = concat!(
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
            "data: [DONE]\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&server)
            .await;

        let client = Client::new(server.uri(), "sk-test", "gpt-4o");
        let mut deltas: Vec<String> = vec![];
        let resp = client
            .complete(&[Message::user("hi")], &[], |d| deltas.push(d.to_string()))
            .await
            .unwrap();

        assert_eq!(deltas.join(""), "Hello");
        assert_eq!(resp.text, "Hello");
        assert!(resp.tool_calls.is_empty());
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 5);
    }

    #[tokio::test]
    async fn complete_detects_tool_calls() {
        let server = MockServer::start().await;
        let sse_body = concat!(
            "data: {\"id\":\"2\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_abc\",\"type\":\"function\",\"function\":{\"name\":\"read_file\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"2\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\":\\\"src/main.rs\\\"}\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"2\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":20,\"completion_tokens\":10}}\n\n",
            "data: [DONE]\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&server)
            .await;

        let client = Client::new(server.uri(), "sk-test", "gpt-4o");
        let resp = client
            .complete(&[Message::user("hi")], &[], |_| {})
            .await
            .unwrap();

        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].function.name, "read_file");
        assert!(resp.tool_calls[0].function.arguments.contains("src/main.rs"));
    }
}
