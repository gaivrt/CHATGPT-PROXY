use axum::{Json, extract::{Extension, ConnectInfo}};
use uuid::Uuid;
use std::sync::Arc;
use std::net::SocketAddr;
use crate::config::AppConfig;
use crate::middleware::SharedRequestTracker;
use crate::openai_types::{ChatCompletionRequest, ChatCompletionResponse, Choice, MessageResponse, Usage};
use crate::proxy_service;
use crate::utils;
use crate::middleware;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;

/// 系统状态信息
#[derive(Serialize)]
pub struct SystemStatus {
    version: &'static str,
    uptime_seconds: u64,
    server_port: u16,
    rate_limits: RateLimits,
    stats: SystemStats,
}

#[derive(Serialize)]
pub struct RateLimits {
    max_requests_per_minute: u32,
    max_tokens_per_minute: u32,
}

#[derive(Serialize)]
pub struct SystemStats {
    total_requests: u64,
    total_tokens: u64,
    active_ips: usize,
}

// 系统启动时间
static mut START_TIME: Option<SystemTime> = None;
// 系统统计数据
static mut TOTAL_REQUESTS: u64 = 0;
static mut TOTAL_TOKENS: u64 = 0;

/// 初始化系统状态
pub fn initialize_system_status() {
    unsafe {
        START_TIME = Some(SystemTime::now());
    }
}

/// 增加请求计数
pub fn increment_request_count() {
    unsafe {
        TOTAL_REQUESTS += 1;
    }
}

/// 增加token计数
pub fn add_tokens(tokens: u64) {
    unsafe {
        TOTAL_TOKENS += tokens;
    }
}

/// 状态页面接口
pub async fn get_status(
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(tracker): Extension<SharedRequestTracker>,
) -> Json<SystemStatus> {
    // 计算运行时间
    let uptime = unsafe {
        START_TIME.map_or(0, |start| {
            SystemTime::now().duration_since(start).unwrap_or_default().as_secs()
        })
    };
    
    // 获取活跃IP数量
    let active_ips = {
        let _tracker = tracker.lock().await;  // 添加下划线前缀忽略未使用变量警告
        // 这里假设RequestTracker有一个方法来获取活跃IP数量
        // 实际实现中可能需要添加此方法
        0 // 暂时返回0，后续可以根据实际情况实现
    };
    
    // 获取统计数据
    let stats = unsafe {
        SystemStats {
            total_requests: TOTAL_REQUESTS,
            total_tokens: TOTAL_TOKENS,
            active_ips,
        }
    };
    
    // 构建状态响应
    let status = SystemStatus {
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        server_port: config.server_port,
        rate_limits: RateLimits {
            max_requests_per_minute: config.max_requests_per_minute,
            max_tokens_per_minute: config.max_tokens_per_minute,
        },
        stats,
    };
    
    Json(status)
}

/// 接收 /v1/chat/completions 的POST请求
pub async fn chat_completion(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(tracker): Extension<SharedRequestTracker>,
    Json(payload): Json<ChatCompletionRequest>,
) -> Json<ChatCompletionResponse> {
    tracing::debug!("Received chat completion request from {}: {:?}", addr, payload);
    
    // 增加请求计数
    increment_request_count();
    
    // 调用代理服务，向 ChatGPT 网页接口发起请求
    let content_result = match proxy_service::send_to_chatgpt(&payload, config.clone()).await {
        Ok(c) => c,
        Err(e) => {
            // 记录错误日志
            tracing::error!("Error in chat completion from {}: {:#}", addr, e);
            
            // 处理错误，返回一个容错的response
            let fallback_resp = ChatCompletionResponse {
                id: format!("chatcmpl-{}", Uuid::new_v4()),
                object: "chat.completion".to_string(),
                created: current_timestamp(),
                choices: vec![Choice {
                    index: 0,
                    message: MessageResponse {
                        role: "assistant".to_string(),
                        content: format!("Error: {:#}", e),
                    },
                    finish_reason: "error".to_string(),
                }],
                usage: None,
            };
            return Json(fallback_resp);
        }
    };

    // 估算token数量
    let prompt_tokens = utils::estimate_token_count(&payload);
    let completion_tokens = utils::estimate_token_count_str(&content_result);
    let total_tokens = prompt_tokens + completion_tokens;
    
    // 增加token计数（转换为u64类型）
    add_tokens(total_tokens as u64);
    
    // 记录token用量（用于限流）
    let _ = middleware::record_token_usage(
        addr.ip(), 
        total_tokens as u32, 
        tracker, 
        config.clone()
    ).await;
    
    // 构建OpenAI兼容格式的响应
    let response = ChatCompletionResponse {
        id: format!("chatcmpl-{}", Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: current_timestamp(),
        choices: vec![
            Choice {
                index: 0,
                message: MessageResponse {
                    role: "assistant".to_string(),
                    content: content_result,
                },
                finish_reason: "stop".to_string(),
            }
        ],
        usage: Some(Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }),
    };

    tracing::debug!("Returning response to {} with {} tokens", addr, total_tokens);
    Json(response)
}

/// 获取当前Unix时间戳(秒)
fn current_timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_secs() as i64
}
