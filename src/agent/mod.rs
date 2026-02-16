mod gemini;
mod openai;

pub use gemini::GeminiAgent;
pub use openai::OpenAiAgent;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
        function_name: String,
        content: String,
    },
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

#[derive(Debug)]
pub struct AgentResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProvider {
    OpenAi,
    Gemini,
}

impl std::str::FromStr for AgentProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" | "gpt" => Ok(AgentProvider::OpenAi),
            "gemini" => Ok(AgentProvider::Gemini),
            _ => Err(format!("unknown provider: '{}'. use 'openai' or 'gemini'", s)),
        }
    }
}

#[async_trait]
pub trait Agent: Send + Sync {
    async fn chat(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
    ) -> Result<AgentResponse, String>;

    /// Same as chat but streams content to `on_chunk` as it arrives (e.g. for live terminal output).
    async fn chat_stream<F>(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
        on_chunk: &mut F,
    ) -> Result<AgentResponse, String>
    where
        F: FnMut(&str) + Send;
}

#[async_trait]
impl Agent for OpenAiAgent {
    async fn chat(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
    ) -> Result<AgentResponse, String> {
        OpenAiAgent::chat(self, messages, user_input).await
    }

    async fn chat_stream<F>(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
        on_chunk: &mut F,
    ) -> Result<AgentResponse, String>
    where
        F: FnMut(&str) + Send,
    {
        OpenAiAgent::chat_stream(self, messages, user_input, on_chunk).await
    }
}

#[async_trait]
impl Agent for GeminiAgent {
    async fn chat(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
    ) -> Result<AgentResponse, String> {
        GeminiAgent::chat(self, messages, user_input).await
    }

    async fn chat_stream<F>(
        &self,
        messages: &mut Vec<Message>,
        user_input: Option<&str>,
        on_chunk: &mut F,
    ) -> Result<AgentResponse, String>
    where
        F: FnMut(&str) + Send,
    {
        GeminiAgent::chat_stream(self, messages, user_input, on_chunk).await
    }
}
