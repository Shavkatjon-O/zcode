use super::{AgentResponse, Message, ToolCall};
use serde::{Deserialize, Serialize};
use std::pin::pin;
use tokio_stream::StreamExt;

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Serialize)]
struct Tool {
    r#type: String,
    function: FunctionDef,
}

#[derive(Debug, Serialize)]
struct FunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

// Streaming chunk (SSE delta)
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Option<Vec<StreamChoice>>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<StreamToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<StreamFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

fn tool_defs() -> Vec<Tool> {
    vec![
        Tool {
            r#type: "function".into(),
            function: FunctionDef {
                name: "create_file".into(),
                description: "Create a new file with the given path and content".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path" },
                        "content": { "type": "string", "description": "File content" }
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        Tool {
            r#type: "function".into(),
            function: FunctionDef {
                name: "read_file".into(),
                description: "Read contents of a file".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path" }
                    },
                    "required": ["path"]
                }),
            },
        },
        Tool {
            r#type: "function".into(),
            function: FunctionDef {
                name: "write_file".into(),
                description: "Write or overwrite file content".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path" },
                        "content": { "type": "string", "description": "File content" }
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        Tool {
            r#type: "function".into(),
            function: FunctionDef {
                name: "list_dir".into(),
                description: "List directory contents".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path" }
                    },
                    "required": ["path"]
                }),
            },
        },
        Tool {
            r#type: "function".into(),
            function: FunctionDef {
                name: "run_command".into(),
                description: "Run a shell command".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Shell command to run" }
                    },
                    "required": ["command"]
                }),
            },
        },
        Tool {
            r#type: "function".into(),
            function: FunctionDef {
                name: "create_directory".into(),
                description: "Create a directory (and parent directories if needed)".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path" }
                    },
                    "required": ["path"]
                }),
            },
        },
    ]
}

const SYSTEM_PROMPT: &str = r#"You are a CLI coding agent that helps developers. You can create files, read files, write files, list directories, run commands, and create directories. Work in the current directory unless told otherwise. Be concise. When creating or editing code, write complete implementations."#;

pub struct OpenAiAgent {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiAgent {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "gpt-4o-mini".into(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Single completion with no tools (e.g. for planning). Returns assistant content text.
    pub async fn completion(&self, system: &str, user: &str) -> Result<String, String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        });

        let resp = self
            .client
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(format!("API error: {}", err_text));
        }

        let chat_resp: ChatResponse = resp.json().await.map_err(|e| e.to_string())?;
        let choice = chat_resp.choices.into_iter().next().ok_or("No response")?;
        Ok(choice.message.content.unwrap_or_default())
    }

    pub async fn chat(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
    ) -> Result<AgentResponse, String> {
        if let Some(input) = user_input {
            messages.push(Message::Role {
                role: "user".into(),
                content: input.into(),
            });
        }

        let mut request_messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "system",
            "content": SYSTEM_PROMPT
        })];

        for m in messages.iter() {
            match m {
                Message::Role { role, content } => {
                    request_messages.push(serde_json::json!({
                        "role": role,
                        "content": content
                    }));
                }
                Message::Assistant {
                    role,
                    content,
                    tool_calls,
                } => {
                    let mut msg = serde_json::json!({
                        "role": role,
                        "content": content
                    });
                    if let Some(tc) = tool_calls {
                        msg["tool_calls"] = serde_json::to_value(tc).unwrap();
                    }
                    request_messages.push(msg);
                }
                Message::ToolResult {
                    role,
                    tool_call_id,
                    function_name: _,
                    content,
                } => {
                    request_messages.push(serde_json::json!({
                        "role": role,
                        "tool_call_id": tool_call_id,
                        "content": content
                    }));
                }
            }
        }

        let body = serde_json::json!({
            "model": self.model,
            "messages": request_messages,
            "tools": tool_defs(),
            "tool_choice": "auto"
        });

        let resp = self
            .client
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(format!("API error: {}", err_text));
        }

        let chat_resp: ChatResponse = resp.json().await.map_err(|e| e.to_string())?;
        let choice = chat_resp.choices.into_iter().next().ok_or("No response")?;
        let msg = choice.message;

        messages.push(Message::Assistant {
            role: "assistant".into(),
            content: msg.content.clone(),
            tool_calls: msg.tool_calls.clone(),
        });

        Ok(AgentResponse {
            content: msg.content,
            tool_calls: msg.tool_calls,
        })
    }

    pub async fn chat_stream<F>(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
        on_chunk: &mut F,
    ) -> Result<AgentResponse, String>
    where
        F: FnMut(&str) + Send,
    {
        if let Some(input) = user_input {
            messages.push(Message::Role {
                role: "user".into(),
                content: input.into(),
            });
        }

        let mut request_messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "system",
            "content": SYSTEM_PROMPT
        })];

        for m in messages.iter() {
            match m {
                Message::Role { role, content } => {
                    request_messages.push(serde_json::json!({
                        "role": role,
                        "content": content
                    }));
                }
                Message::Assistant {
                    role,
                    content,
                    tool_calls,
                } => {
                    let mut msg = serde_json::json!({
                        "role": role,
                        "content": content
                    });
                    if let Some(tc) = tool_calls {
                        msg["tool_calls"] = serde_json::to_value(tc).unwrap();
                    }
                    request_messages.push(msg);
                }
                Message::ToolResult {
                    role,
                    tool_call_id,
                    function_name: _,
                    content,
                } => {
                    request_messages.push(serde_json::json!({
                        "role": role,
                        "tool_call_id": tool_call_id,
                        "content": content
                    }));
                }
            }
        }

        let body = serde_json::json!({
            "model": self.model,
            "messages": request_messages,
            "tools": tool_defs(),
            "tool_choice": "auto",
            "stream": true
        });

        let resp = self
            .client
            .post(API_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(format!("API error: {}", err_text));
        }

        let mut stream = pin!(resp.bytes_stream());
        let mut buffer = Vec::<u8>::new();
        let mut content_acc = String::new();
        // Accumulate tool calls by index: id, name, arguments (append for arguments)
        let mut tool_calls_acc: Vec<(String, String, String)> = Vec::new();

        'stream: while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| e.to_string())?;
            buffer.extend_from_slice(&chunk);

            // Process complete lines (SSE: "data: {...}\n" or "data: [DONE]\n")
            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = std::mem::take(&mut buffer);
                let (full_line, rest) = line.split_at(pos + 1);
                buffer.extend_from_slice(rest);

                let line_str = match std::str::from_utf8(full_line) {
                    Ok(s) => s.trim(),
                    Err(_) => continue,
                };
                let Some(data) = line_str.strip_prefix("data: ") else {
                    continue;
                };
                if data == "[DONE]" {
                    break 'stream;
                }
                let Ok(stream_chunk) = serde_json::from_str::<StreamChunk>(data) else {
                    continue;
                };
                let Some(choices) = stream_chunk.choices else {
                    continue;
                };
                let Some(choice) = choices.into_iter().next() else {
                    continue;
                };
                let delta = choice.delta;

                if let Some(ref text) = delta.content {
                    if !text.is_empty() {
                        on_chunk(text);
                        content_acc.push_str(text);
                    }
                }
                if let Some(deltas) = delta.tool_calls {
                    for d in deltas {
                        let idx = d.index;
                        if idx >= tool_calls_acc.len() {
                            tool_calls_acc.resize(idx + 1, (String::new(), String::new(), String::new()));
                        }
                        let acc = &mut tool_calls_acc[idx];
                        if let Some(id) = d.id {
                            acc.0 = id;
                        }
                        if let Some(f) = d.function {
                            if let Some(n) = f.name {
                                acc.1 = n;
                            }
                            if let Some(a) = f.arguments {
                                acc.2.push_str(&a);
                            }
                        }
                    }
                }
            }
        }

        // Build final tool_calls from accumulator
        let tool_calls: Option<Vec<ToolCall>> = if tool_calls_acc.is_empty() {
            None
        } else {
            Some(
                tool_calls_acc
                    .into_iter()
                    .enumerate()
                    .map(|(i, (id, name, arguments))| ToolCall {
                        id: if id.is_empty() {
                            format!("call_{}", i)
                        } else {
                            id
                        },
                        type_: "function".into(),
                        function: super::FunctionCall { name, arguments },
                    })
                    .collect(),
            )
        };

        let content = if content_acc.is_empty() {
            None
        } else {
            Some(content_acc)
        };

        messages.push(Message::Assistant {
            role: "assistant".into(),
            content: content.clone(),
            tool_calls: tool_calls.clone(),
        });

        Ok(AgentResponse { content, tool_calls })
    }
}
