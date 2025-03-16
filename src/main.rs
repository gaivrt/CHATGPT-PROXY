use axum::{Router, routing::{post, get}, Extension};
use axum::extract::connect_info::ConnectInfo;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use axum::http::Request;
use std::path::Path;
use std::env;

mod config;
mod handlers;
mod proxy_service;
mod openai_types;
mod utils;
mod token_refresher;
mod middleware;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "chatgpt_proxy=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. 加载配置（.env）
    // 检查当前目录下的 .env 文件
    let env_path = Path::new(".env");
    if env_path.exists() {
        match dotenvy::from_path(env_path) {
            Ok(_) => tracing::info!("Loaded .env file successfully"),
            Err(e) => tracing::error!("Failed to load .env file: {}", e),
        }
    } else {
        tracing::warn!(".env file not found in current directory");
    }

    // 打印关键环境变量的状态（不打印具体值，仅检查是否存在）
    tracing::info!("CHATGPT_SESSION_TOKEN exists: {}", env::var("CHATGPT_SESSION_TOKEN").is_ok());
    tracing::info!("CHATGPT_AUTHORIZATION exists: {}", env::var("CHATGPT_AUTHORIZATION").is_ok());
    
    let config = Arc::new(config::AppConfig::from_env()?);
    let server_port = config.server_port;  // 提前获取端口号
    tracing::info!("Configuration loaded successfully");
    
    // 3. 初始化系统状态追踪
    handlers::initialize_system_status();
    
    // 4. 初始化Token刷新器
    let refreshable_config = Arc::new(Mutex::new(config.as_ref().clone()));
    let token_refresher = Arc::new(
        token_refresher::TokenRefresher::new(refreshable_config.clone())
            .with_check_interval(Duration::from_secs(60 * 60)) // 每小时检查一次
    );
    
    // 启动后台Token刷新任务
    token_refresher.clone().start_background_refresh().await;
    tracing::info!("Token refresher background task started");

    // 5. 创建请求跟踪器用于速率限制
    let request_tracker = middleware::create_request_tracker();
    tracing::info!("Request rate limiter initialized");

    // 6. 构建路由
    let app = Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completion))
        .route("/health", get(|| async { "OK" }))
        .route("/status", get(handlers::get_status))
        .layer(Extension(config.clone()))
        .layer(Extension(request_tracker.clone()))
        .layer(tower::ServiceBuilder::new()
            .layer(axum::middleware::from_fn(move |req: Request<axum::body::Body>, next| {
                let tracker = request_tracker.clone();
                let config = config.clone();
                async move {
                    let connect_info = req.extensions().get::<ConnectInfo<SocketAddr>>().cloned().unwrap();
                    middleware::rate_limiter(connect_info, req, next, tracker, config).await
                }
            }))
        );

    // 7. 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], server_port));
    let display_ip = if addr.ip().is_unspecified() {
        "127.0.0.1".to_string()
    } else {
        addr.ip().to_string()
    };
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("Status page available at http://{}:{}/status", display_ip, addr.port());

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}
