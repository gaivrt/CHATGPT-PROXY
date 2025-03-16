use serde::{Deserialize, Serialize};

/// ChatGPT请求体 - 与官方OpenAI API兼容
#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    #[allow(dead_code)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[allow(dead_code)]
    pub frequency_penalty: Option<f64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub presence_penalty: Option<f64>,
    // 可根据需要扩展更多字段
}

/// 用户 / 系统 / 助手消息
#[derive(Debug, Deserialize)]
pub struct Message {
    #[allow(dead_code)]
    pub role: String,   // "user", "assistant", "system"
    pub content: String,
}

/// ChatGPT响应体 - 与官方OpenAI API兼容
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    #[serde(rename = "object")]
    pub object: String,
    pub created: i64,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize)]
pub struct Choice {
    pub index: usize,
    pub message: MessageResponse,
    pub finish_reason: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}
