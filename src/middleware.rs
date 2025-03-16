use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::net::IpAddr;

use axum::{
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tokio::sync::Mutex;

use crate::config::AppConfig;

// 用于跟踪IP地址请求数量的结构
pub struct RequestTracker {
    requests: HashMap<IpAddr, Vec<Instant>>,
    token_usage: HashMap<IpAddr, (Instant, u32)>, // (上次计数时间, token数量)
}

impl RequestTracker {
    fn new() -> Self {
        Self {
            requests: HashMap::new(),
            token_usage: HashMap::new(),
        }
    }

    // 检查请求频率并记录
    fn check_and_record_request(&mut self, ip: IpAddr, config: &AppConfig, now: Instant) -> bool {
        // 清理过期的请求记录
        let window = Duration::from_secs(60); // 1分钟窗口
        
        // 获取或创建该IP的请求记录
        let requests = self.requests.entry(ip).or_insert_with(Vec::new);
        
        // 删除1分钟前的记录
        requests.retain(|&time| now.duration_since(time) < window);
        
        // 检查是否超过限制
        if requests.len() >= config.max_requests_per_minute as usize {
            return false;
        }
        
        // 记录本次请求
        requests.push(now);
        true
    }
    
    // 记录token用量
    fn record_token_usage(&mut self, ip: IpAddr, tokens: u32, config: &AppConfig, now: Instant) -> bool {
        let window = Duration::from_secs(60); // 1分钟窗口
        
        // 获取或创建该IP的token用量记录
        let (last_time, token_count) = self.token_usage.entry(ip).or_insert((now, 0));
        
        // 如果已经过了一分钟，重置计数
        if now.duration_since(*last_time) >= window {
            *last_time = now;
            *token_count = tokens;
            return true;
        }
        
        // 检查是否超过限制
        if *token_count + tokens > config.max_tokens_per_minute {
            return false;
        }
        
        // 累加token用量
        *token_count += tokens;
        true
    }
}

// 创建一个全局请求跟踪器
pub type SharedRequestTracker = Arc<Mutex<RequestTracker>>;

pub fn create_request_tracker() -> SharedRequestTracker {
    Arc::new(Mutex::new(RequestTracker::new()))
}

// 使用异步中间件处理函数的方式定义中间件
pub async fn rate_limiter<B>(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    req: Request<B>,
    next: Next<B>,
    tracker: Arc<Mutex<RequestTracker>>,
    config: Arc<AppConfig>,
) -> Result<Response, StatusCode> {
    let ip = addr.ip();
    let now = Instant::now();
    
    // 锁定追踪器并检查请求频率
    let is_allowed = {
        let mut tracker = tracker.lock().await;
        tracker.check_and_record_request(ip, &config, now)
    };
    
    if !is_allowed {
        tracing::warn!("Rate limit exceeded for IP: {}", ip);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    // 继续处理请求
    Ok(next.run(req).await)
}

// 记录token用量的函数
pub async fn record_token_usage(
    ip: IpAddr, 
    tokens: u32,
    tracker: SharedRequestTracker,
    config: Arc<AppConfig>,
) -> bool {
    let now = Instant::now();
    let mut tracker = tracker.lock().await;
    tracker.record_token_usage(ip, tokens, &config, now)
}