use super::{AgentResponse, Message, ToolCall};
use serde::Deserialize;

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const MODEL: &str = "gemini-2.0-flash";

const SYSTEM_PROMPT: &str = r#"You are a CLI coding agent that helps developers. You can create files, read files, write files, list directories, run commands, and create directories. Work in the current directory unless told otherwise. Be concise. When creating or editing code, write complete implementations."#;

fn gemini_tool_defs() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "create_file",
            "description": "Create a new file with the given path and content",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "content": { "type": "string", "description": "File content" }
                },
                "required": ["path", "content"]
            }
        }),
        serde_json::json!({
            "name": "read_file",
            "description": "Read contents of a file",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "name": "write_file",
            "description": "Write or overwrite file content",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "content": { "type": "string", "description": "File content" }
                },
                "required": ["path", "content"]
            }
        }),
        serde_json::json!({
            "name": "list_dir",
            "description": "List directory contents",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path" }
                },
                "required": ["path"]
            }
        }),
        serde_json::json!({
            "name": "run_command",
            "description": "Run a shell command",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to run" }
                },
                "required": ["command"]
            }
        }),
        serde_json::json!({
            "name": "create_directory",
            "description": "Create a directory (and parent directories if needed)",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path" }
                },
                "required": ["path"]
            }
        }),
    ]
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<Content>,
    #[serde(default)]
    _finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct Content {
    parts: Option<Vec<Part>>,
    #[serde(default)]
    _role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Part {
    text: Option<String>,
    #[serde(rename = "functionCall")]
    function_call: Option<FunctionCallPart>,
}

#[derive(Debug, Deserialize)]
struct FunctionCallPart {
    name: String,
    args: serde_json::Value,
}

pub struct GeminiAgent {
    client: reqwest::Client,
    api_key: String,
}

impl GeminiAgent {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    fn message_to_contents(
        messages: &[Message],
        user_input: Option<&str>,
    ) -> Vec<serde_json::Value> {
        let mut contents = vec![];

        for m in messages.iter() {
            match m {
                Message::Role { role, content } => {
                    let gemini_role = if role == "user" { "user" } else { "user" };
                    contents.push(serde_json::json!({
                        "role": gemini_role,
                        "parts": [{"text": content}]
                    }));
                }
                Message::Assistant {
                    content,
                    tool_calls,
                    ..
                } => {
                    let mut parts: Vec<serde_json::Value> = vec![];
                    if let Some(c) = content.as_ref().filter(|s| !s.is_empty()) {
                        parts.push(serde_json::json!({"text": c}));
                    }
                    if let Some(tc) = tool_calls {
                        for t in tc {
                            parts.push(serde_json::json!({
                                "functionCall": {
                                    "name": t.function.name,
                                    "args": serde_json::from_str::<serde_json::Value>(&t.function.arguments).unwrap_or(serde_json::json!({}))
                                }
                            }));
                        }
                    }
                    if !parts.is_empty() {
                        contents.push(serde_json::json!({
                            "role": "model",
                            "parts": parts
                        }));
                    }
                }
                Message::ToolResult {
                    function_name,
                    content,
                    ..
                } => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{
                            "functionResponse": {
                                "name": function_name,
                                "response": {"result": content}
                            }
                        }]
                    }));
                }
            }
        }

        if let Some(input) = user_input {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{"text": input}]
            }));
        }

        contents
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

        let contents = Self::message_to_contents(messages, None);

        let body = serde_json::json!({
            "contents": contents,
            "systemInstruction": {
                "parts": [{"text": SYSTEM_PROMPT}]
            },
            "tools": [{
                "functionDeclarations": gemini_tool_defs()
            }],
            "generationConfig": {
                "temperature": 0.1,
                "topP": 0.95,
                "maxOutputTokens": 8192
            }
        });

        let url = format!(
            "{}/{}:generateContent?key={}",
            API_BASE, MODEL, self.api_key
        );

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        let resp_text = resp.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, resp_text));
        }

        let gen_resp: GenerateContentResponse =
            serde_json::from_str(&resp_text).map_err(|e| e.to_string())?;

        if let Some(err) = gen_resp.error {
            return Err(format!("API error: {}", err.message));
        }

        let candidate = gen_resp
            .candidates
            .and_then(|c| c.into_iter().next())
            .ok_or("No response from model")?;

        let content = candidate.content.ok_or("Empty candidate content")?;
        let parts = content.parts.unwrap_or_default();

        let mut response_content: Option<String> = None;
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            if let Some(text) = &part.text {
                response_content = Some(text.clone());
            }
            if let Some(fc) = &part.function_call {
                let args_str =
                    serde_json::to_string(&fc.args).unwrap_or_else(|_| "{}".to_string());
                tool_calls.push(ToolCall {
                    id: format!("gemini-{}", i),
                    type_: "function".into(),
                    function: super::FunctionCall {
                        name: fc.name.clone(),
                        arguments: args_str,
                    },
                });
            }
        }

        if !tool_calls.is_empty() {
            messages.push(Message::Assistant {
                role: "assistant".into(),
                content: response_content.clone(),
                tool_calls: Some(tool_calls.clone()),
            });
            return Ok(AgentResponse {
                content: None,
                tool_calls: Some(tool_calls),
            });
        }

        messages.push(Message::Assistant {
            role: "assistant".into(),
            content: response_content.clone(),
            tool_calls: None,
        });

        Ok(AgentResponse {
            content: response_content,
            tool_calls: None,
        })
    }
}
