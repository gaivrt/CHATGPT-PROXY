use anyhow::{anyhow, Result};
use reqwest::{Client, header, Proxy};
use std::sync::Arc;
use uuid::Uuid;
use crate::config::AppConfig;
use crate::openai_types::ChatCompletionRequest;

/// 发送请求到ChatGPT网页API
pub async fn send_to_chatgpt(req_payload: &ChatCompletionRequest, config: Arc<AppConfig>) -> Result<String> {
    // 1. 首先，我们尝试获取访问令牌
    let access_token = get_access_token(&config).await?;
    tracing::debug!("成功获取访问令牌");

    // 2. 构造ChatGPT网页端所需的payload
    let message_id = Uuid::new_v4().to_string();
    let parent_message_id = Uuid::new_v4().to_string();
    
    // 将OpenAI API格式转换为ChatGPT网页端格式
    let chatgpt_payload = serde_json::json!({
        "action": "next",
        "messages": [
            {
                "id": message_id,
                "role": "user",
                "content": {
                    "content_type": "text",
                    "parts": req_payload
                        .messages
                        .iter()
                        .map(|m| m.content.clone())
                        .collect::<Vec<String>>(),
                }
            }
        ],
        "model": map_model_name(&req_payload.model),
        "conversation_id": null,
        "parent_message_id": parent_message_id,
        "temperature": req_payload.temperature.unwrap_or(0.7),
        "top_p": req_payload.top_p.unwrap_or(1.0),
    });

    // 3. 构造请求头
    let mut headers = header::HeaderMap::new();
    
    // 使用 access_token 设置 Authorization
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("Bearer {}", access_token))
            .map_err(|e| anyhow!("Invalid authorization header: {}", e))?
    );
    
    // 添加其他必要的头部，模拟真实浏览器访问以绕过 Cloudflare
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json")
    );
    
    // 更真实的 User-Agent
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    );
    
    headers.insert(
        header::HeaderName::from_static("accept"),
        header::HeaderValue::from_static("text/event-stream")
    );
    
    // 添加关键的 Cloudflare 相关头部
    headers.insert(
        header::REFERER,
        header::HeaderValue::from_static("https://chat.openai.com/")
    );
    
    headers.insert(
        header::ORIGIN,
        header::HeaderValue::from_static("https://chat.openai.com")
    );
    
    headers.insert(
        header::HeaderName::from_static("sec-ch-ua"),
        header::HeaderValue::from_static("\"Not_A Brand\";v=\"99\", \"Google Chrome\";v=\"120\", \"Chromium\";v=\"120\"")
    );
    
    headers.insert(
        header::HeaderName::from_static("sec-ch-ua-mobile"),
        header::HeaderValue::from_static("?0")
    );
    
    headers.insert(
        header::HeaderName::from_static("sec-ch-ua-platform"),
        header::HeaderValue::from_static("\"Windows\"")
    );
    
    headers.insert(
        header::HeaderName::from_static("sec-fetch-dest"),
        header::HeaderValue::from_static("empty")
    );
    
    headers.insert(
        header::HeaderName::from_static("sec-fetch-mode"),
        header::HeaderValue::from_static("cors")
    );
    
    headers.insert(
        header::HeaderName::from_static("sec-fetch-site"),
        header::HeaderValue::from_static("same-origin")
    );
    
    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8")
    );
    
    // 添加必要的 Cookie，包括 cf_clearance 来绕过 Cloudflare
    let cf_cookie = match std::env::var("CF_CLEARANCE") {
        Ok(cf) => cf,
        Err(_) => "".to_string()
    };
    
    let cookie_value = format!(
        "__Secure-next-auth.session-token={}; cf_clearance={}; __Secure-next-auth.callback-url=https://chat.openai.com/",
        config.chatgpt_session_token, cf_cookie
    );
    
    headers.insert(
        header::COOKIE,
        header::HeaderValue::from_str(&cookie_value)
            .map_err(|e| anyhow!("Invalid cookie value: {}", e))?
    );

    // 4. 创建客户端并发送请求 - 添加代理支持
    let mut client_builder = Client::builder()
        .cookie_store(true)
        .danger_accept_invalid_certs(true);  // 某些代理可能需要这个选项
        
    // 检查是否存在代理配置，如果有则添加代理
    if let Some(proxy_url) = std::env::var("HTTP_PROXY").ok().or(std::env::var("http_proxy").ok()) {
        tracing::info!("使用HTTP代理: {}", proxy_url);
        let proxy = Proxy::http(&proxy_url)?;
        client_builder = client_builder.proxy(proxy);
    } else if let Some(proxy_url) = std::env::var("HTTPS_PROXY").ok().or(std::env::var("https_proxy").ok()) {
        tracing::info!("使用HTTPS代理: {}", proxy_url);
        let proxy = Proxy::https(&proxy_url)?;
        client_builder = client_builder.proxy(proxy);
    } else if let Some(proxy_url) = std::env::var("ALL_PROXY").ok().or(std::env::var("all_proxy").ok()) {
        tracing::info!("使用ALL代理: {}", proxy_url);
        // 根据URL判断是http还是https
        if proxy_url.starts_with("http://") {
            let proxy = Proxy::http(&proxy_url)?;
            client_builder = client_builder.proxy(proxy);
        } else if proxy_url.starts_with("https://") {
            let proxy = Proxy::https(&proxy_url)?;
            client_builder = client_builder.proxy(proxy);
        } else {
            // 尝试添加前缀
            let proxy = Proxy::http(&format!("http://{}", proxy_url))?;
            client_builder = client_builder.proxy(proxy);
        }
    } else {
        // 尝试使用默认本地代理设置
        let proxies = [
            "http://127.0.0.1:10809",  // 常见 v2rayN 端口
            "http://127.0.0.1:7890",   // 常见 Clash 端口
            "http://127.0.0.1:1080",   // 常见 SOCKS 端口
            "http://127.0.0.1:8080",   // 常见通用端口
        ];
        
        for proxy_url in proxies {
            match Proxy::http(proxy_url) {
                Ok(p) => {
                    tracing::info!("成功设置HTTP代理: {}", proxy_url);
                    client_builder = client_builder.proxy(p);
                    break;
                },
                Err(_) => continue,
            }
        }
    }
    
    let client = client_builder.build()?;

    // 尝试绕过 Cloudflare 的其他 API 端点
    // ChatGPT可能有几个API端点，如果一个不行可以尝试另一个
    let api_endpoints = [
        "https://chat.openai.com/backend-api/conversation",
        "https://chat.openai.com/api/conversation",
    ];
    
    let mut last_error = None;
    
    for url in api_endpoints {
        tracing::debug!("尝试API端点: {}", url);
        tracing::debug!("载荷: {}", serde_json::to_string_pretty(&chatgpt_payload)?);
        
        let resp_result = client
            .post(url)
            .headers(headers.clone())
            .json(&chatgpt_payload)
            .send()
            .await;
            
        match resp_result {
            Ok(resp) => {
                // 检查响应状态码
                if resp.status().is_success() {
                    tracing::info!("成功连接到API端点: {}", url);
                    
                    // 解析返回结果
                    let resp_text = resp.text().await?;
                    
                    if resp_text.is_empty() {
                        tracing::warn!("响应为空，尝试下一个端点");
                        last_error = Some(anyhow!("响应为空"));
                        continue;
                    }
                    
                    tracing::debug!("收到来自ChatGPT的回复");
                    
                    // 解析ChatGPT响应，提取所需的内容
                    return parse_chatgpt_response(&resp_text);
                } else {
                    let status = resp.status();
                    let error_text = resp.text().await?;
                    tracing::error!("API错误，端点 {}: 状态 {}, 内容: {}", url, status, error_text);
                    
                    if status.as_u16() == 403 {
                        tracing::error!("遇到Cloudflare保护，尝试下一个端点");
                    }
                    
                    last_error = Some(anyhow!("API错误: 状态 {} {}, 消息: {}", 
                        status.as_u16(), status.canonical_reason().unwrap_or("Unknown"), 
                        error_text));
                }
            },
            Err(e) => {
                tracing::error!("请求失败，端点 {}: {}", url, e);
                last_error = Some(anyhow!("请求失败: {}", e));
            }
        }
    }
    
    // 如果所有端点都失败了，返回最后一个错误
    Err(last_error.unwrap_or_else(|| anyhow!("所有API端点都失败了")))
}

/// 从配置中获取访问令牌
/// 现在的ChatGPT认证流程可能需要多步骤
async fn get_access_token(config: &AppConfig) -> Result<String> {
    // 1. 首先尝试直接使用配置中的授权令牌
    tracing::info!("尝试获取访问令牌...");
    
    if config.chatgpt_authorization.starts_with("Bearer ") {
        tracing::debug!("使用Bearer前缀的授权令牌");
        // 直接使用Bearer令牌
        return Ok(config.chatgpt_authorization.trim_start_matches("Bearer ").to_string());
    } else if config.chatgpt_authorization.starts_with("eyJ") {
        tracing::debug!("使用JWT格式的授权令牌");
        // 看起来已经是JWT格式，直接使用
        return Ok(config.chatgpt_authorization.to_string());
    }
    
    // 2. 如果不是典型的令牌格式，尝试获取新令牌
    tracing::info!("尝试使用会话令牌获取新的访问令牌");
    
    // 创建客户端，同样添加代理支持
    let mut client_builder = Client::builder()
        .cookie_store(true);
        
    // 检查是否存在代理配置
    if let Some(proxy_url) = std::env::var("HTTP_PROXY").ok().or(std::env::var("http_proxy").ok()) {
        let proxy = Proxy::http(&proxy_url)?;
        client_builder = client_builder.proxy(proxy);
    } else if let Some(proxy_url) = std::env::var("HTTPS_PROXY").ok().or(std::env::var("https_proxy").ok()) {
        let proxy = Proxy::https(&proxy_url)?;
        client_builder = client_builder.proxy(proxy);
    } else {
        // 尝试使用默认本地代理
        let proxy_url = "http://127.0.0.1:10809";
        if let Ok(proxy) = Proxy::http(proxy_url) {
            client_builder = client_builder.proxy(proxy);
        }
    }
    
    let client = client_builder.build()?;
    
    // 设置会话Cookie
    let cookie = format!("__Secure-next-auth.session-token={}", config.chatgpt_session_token);
    let mut headers = header::HeaderMap::new();
    headers.insert(header::COOKIE, header::HeaderValue::from_str(&cookie)?);
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    );
    
    // 访问会话端点以获取访问令牌
    tracing::debug!("请求 /api/auth/session 端点");
    let resp = client
        .get("https://chat.openai.com/api/auth/session")
        .headers(headers)
        .send()
        .await?;
    
    let status = resp.status();
    tracing::debug!("会话端点响应状态码: {}", status);
    
    if !status.is_success() {
        let error_text = resp.text().await?;
        tracing::error!("获取访问令牌失败: 状态 {}, 内容: {}", status, error_text);
        return Err(anyhow!("获取访问令牌失败: 状态 {}, 内容: {}", status, error_text));
    }
    
    let session_text = resp.text().await?;
    tracing::debug!("会话响应: {}", session_text);
    
    let json = match serde_json::from_str::<serde_json::Value>(&session_text) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("解析会话响应失败: {}", e);
            return Err(anyhow!("解析会话响应失败: {}", e));
        }
    };
    
    tracing::debug!("会话JSON: {}", json);
    
    // 提取访问令牌
    if let Some(access_token) = json.get("accessToken").and_then(|t| t.as_str()) {
        tracing::info!("成功获取访问令牌");
        return Ok(access_token.to_string());
    }
    
    // 3. 如果上述方法都失败，返回原始授权令牌
    tracing::warn!("无法获取新的访问令牌，使用原始授权令牌");
    tracing::warn!("请确保你的会话令牌是最新的，并且你已经登录到 chat.openai.com");
    
    Ok(config.chatgpt_authorization.to_string())
}

/// 将OpenAI API模型名称映射到ChatGPT网页端支持的模型名称
fn map_model_name(model_name: &str) -> String {
    match model_name {
        // 旧模型映射
        "gpt-3.5-turbo" | "gpt-3.5-turbo-0613" | "gpt-3.5-turbo-16k" | "gpt-3.5-turbo-16k-0613" => 
            "text-davinci-002-render-sha".to_string(),
        "gpt-4" | "gpt-4-0613" => "gpt-4".to_string(),
        "gpt-4-32k" | "gpt-4-32k-0613" => "gpt-4-32k".to_string(),
        
        // 新增模型映射
        "gpt-4o" => "gpt-4o".to_string(), // 适用于大多数问题
        "gpt-4o-mini" => "gpt-4o-mini".to_string(), // 更快地回答大多数问题
        "gpt-4.5" => "gpt-4.5-preview".to_string(), // 研究预览版，擅长写作和构思想法
        "gpt-4.5-preview" => "gpt-4.5-preview".to_string(), // 研究预览版
        "o1" => "o1".to_string(), // 使用高级推理
        "o1-pro" => "o1-pro".to_string(), // 擅长模糊逻辑推理
        "o3-mini" => "o3-mini".to_string(), // 快速进行高级推理
        "o3-mini-high" => "o3-mini-high".to_string(), // 擅长编码和逻辑
        "gpt-4-turbo" => "gpt-4-turbo".to_string(), // 传统模型推理
        
        // 未知模型直接返回原名
        _ => model_name.to_string(), 
    }
}

/// 解析ChatGPT网页端返回的响应，提取有用内容
fn parse_chatgpt_response(response_text: &str) -> Result<String> {
    tracing::debug!("原始响应前100个字符: {}", &response_text.chars().take(100).collect::<String>());
    
    // 检查响应是否为空
    if response_text.is_empty() {
        return Err(anyhow!("Empty response from ChatGPT"));
    }

    // 如果响应是标准的JSON格式
    if response_text.starts_with("{") {
        match serde_json::from_str::<serde_json::Value>(response_text) {
            Ok(json) => {
                tracing::debug!("直接解析JSON响应");
                // 提取消息内容 - 尝试多种可能的路径
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(parts) = content.get("parts") {
                            if let Some(text) = parts.get(0).and_then(|p| p.as_str()) {
                                return Ok(text.to_string());
                            }
                        }
                        // 如果直接有content值
                        if let Some(text) = content.as_str() {
                            return Ok(text.to_string());
                        }
                    }
                }
                
                // 尝试其他可能的路径
                if let Some(content) = json.get("content") {
                    if let Some(text) = content.as_str() {
                        return Ok(text.to_string());
                    }
                }
                
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    return Ok(text.to_string());
                }
                
                // 如果找不到特定路径，返回整个JSON字符串
                return Ok(json.to_string());
            },
            Err(e) => {
                tracing::warn!("JSON解析失败: {}", e);
            }
        }
    }
    
    // 处理SSE格式 (data: 开头的行)
    let lines: Vec<&str> = response_text.lines().collect();
    let mut last_json = "";
    let mut complete_response = String::new();
    
    // 尝试处理所有data行
    for line in lines.iter() {
        if line.starts_with("data:") && *line != "data: [DONE]" {
            let content = line.trim_start_matches("data: ");
            tracing::debug!("找到data行: {}", &content.chars().take(30).collect::<String>());
            
            // 尝试解析为JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                last_json = content;
                
                // 尝试提取消息部分
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(parts) = content.get("parts") {
                            if let Some(text) = parts.get(0).and_then(|p| p.as_str()) {
                                complete_response.push_str(text);
                            }
                        }
                    }
                }
            } else {
                // 如果不是有效的JSON，可能是纯文本
                complete_response.push_str(content);
            }
        }
    }
    
    // 如果找到了完整的响应内容
    if !complete_response.is_empty() {
        return Ok(complete_response);
    }
    
    // 如果解析所有data行仍未找到内容，但存在最后一个有效JSON
    if !last_json.is_empty() {
        match serde_json::from_str::<serde_json::Value>(last_json) {
            Ok(json) => {
                tracing::debug!("尝试从最后一个JSON提取内容");
                // 尝试提取消息内容 (和上面类似)
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(parts) = content.get("parts") {
                            if let Some(text) = parts.get(0).and_then(|p| p.as_str()) {
                                return Ok(text.to_string());
                            }
                        }
                    }
                }
                
                // 如果解析失败，返回原始JSON字符串
                return Ok(json.to_string());
            },
            Err(e) => {
                tracing::error!("解析最后一个JSON失败: {}", e);
            }
        }
    }
    
    // 如果仍然找不到内容，返回原始响应的部分内容
    let preview = if response_text.len() > 1000 {
        format!("{}... (截断)", &response_text[..1000])
    } else {
        response_text.to_string()
    };
    
    tracing::warn!("无法解析ChatGPT响应，返回原始内容预览: {}", preview);
    Ok(format!("无法解析响应。原始内容: {}", preview))
}
