use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Message {
    Role { role: String, content: String },
    Assistant {
        role: String,
        content: Option<String>,
        tool_calls: Option<Vec<ToolCall>>,
    },
    ToolResult {
        role: String,
        tool_call_id: String,
        content: String,
    },
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type", default = "default_tool_type")]
    pub type_: String,
    pub function: FunctionCall,
}

fn default_tool_type() -> String {
    "function".into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
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

pub struct ChatGptAgent {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl ChatGptAgent {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "gpt-4o-mini".into(),
        }
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
}

#[derive(Debug)]
pub struct AgentResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
