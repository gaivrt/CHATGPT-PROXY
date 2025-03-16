use crate::openai_types::ChatCompletionRequest;

/// 估算请求中的token数量（粗略估计）
pub fn estimate_token_count(req: &ChatCompletionRequest) -> i64 {
    let mut tokens = 0;
    
    // 每条消息的基础token数
    let message_tokens = 4; // 每条消息的元数据约占用4个token
    
    for message in &req.messages {
        // 内容的token数：粗略估计为字符数/4（大约是英文单词的平均长度）
        let content_tokens = message.content.chars().count() as i64 / 4;
        tokens += message_tokens + content_tokens;
    }
    
    // 考虑请求的基础token数
    tokens += 8; // 请求元数据约占8个token
    
    tokens
}

/// 估算字符串内容的token数量（粗略估计）
pub fn estimate_token_count_str(text: &str) -> i64 {
    // 粗略估计为字符数/4
    text.chars().count() as i64 / 4
}
