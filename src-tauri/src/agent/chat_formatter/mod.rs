use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct LlmToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

pub trait ChatFormatter: Send + Sync {
    fn format_messages(
        &self,
        messages: &[LlmMessage],
        system_prompt: &str,
    ) -> Result<Value, String>;

    fn format_tools(&self, tools: &[Value]) -> Result<Value, String>;

    fn parse_response(&self, response: &Value) -> Result<ParseResult, String>;

    fn parse_chunk(&self, chunk: &Value) -> Result<ChunkResult, String>;
}

pub enum ParseResult {
    Content(String),
    ToolCalls(Vec<LlmToolCall>),
}

pub struct ChunkResult {
    pub content_delta: Option<String>,
    pub thinking_delta: Option<String>,
    pub tool_calls: Vec<LlmToolCall>,
    pub finish_reason: Option<String>,
}

pub mod openai;
pub mod anthropic;
pub use openai::OpenAIChatFormatter;
pub use anthropic::AnthropicChatFormatter;
