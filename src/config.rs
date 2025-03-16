use std::env;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppConfig {
    // 认证信息
    pub chatgpt_session_token: String, 
    pub chatgpt_authorization: String, 
    
    // 服务器设置
    pub server_port: u16,
    
    // 限流设置（可选）
    pub max_requests_per_minute: u32,
    pub max_tokens_per_minute: u32,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let chatgpt_session_token =
            env::var("CHATGPT_SESSION_TOKEN").expect("Missing CHATGPT_SESSION_TOKEN in env");
        let chatgpt_authorization =
            env::var("CHATGPT_AUTHORIZATION").expect("Missing CHATGPT_AUTHORIZATION in env");
        
        // 服务器端口，默认为3000
        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000);
            
        // 可选配置，使用默认值
        let max_requests_per_minute = env::var("MAX_REQUESTS_PER_MINUTE")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60);
            
        let max_tokens_per_minute = env::var("MAX_TOKENS_PER_MINUTE")
            .unwrap_or_else(|_| "40000".to_string())
            .parse()
            .unwrap_or(40000);
        
        Ok(Self {
            chatgpt_session_token,
            chatgpt_authorization,
            server_port,
            max_requests_per_minute,
            max_tokens_per_minute,
        })
    }
    
    // 已删除未使用的 is_valid 方法
}
