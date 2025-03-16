# ChatGPT-Proxy

使用 Rust 实现的 ChatGPT 反向代理服务，允许使用已有的 ChatGPT Pro 账号实现完全兼容官方 OpenAI API 的接口调用。

## 📌 项目概述

本项目提供了一个反向代理服务，允许你：

- 使用已有的 ChatGPT Pro 账号调用兼容 OpenAI API 格式的接口
- 无需单独购买官方 API 额度
- 获得与官方 API 完全一致的调用体验
- 支持最新的 GPT-4o、o1 等所有 ChatGPT 模型

## 🧩 功能特点

- **完全兼容**：与 OpenAI 官方 API 格式完全兼容
- **高性能**：基于 Rust 和 Axum 构建的高效 HTTP 服务
- **简单配置**：使用 .env 文件实现简单配置
- **代理支持**：内置 HTTP 代理支持，方便国内用户访问
- **全模型支持**：支持所有最新的 ChatGPT 模型
- **灵活扩展**：模块化设计，易于扩展
- **安全防护**：支持请求限流

## 🚀 快速开始

### 前置需求

- Rust 及 Cargo (推荐 1.60+)
- 有效的 ChatGPT Pro 账号
- (可选) HTTP 代理，用于访问 OpenAI 服务

### 安装步骤

1. 克隆仓库：
```bash
git clone https://github.com/gaivrt/CHATGPT-PROXY.git
cd CHATGPT-PROXY
```

2. 复制环境变量配置文件：
```bash
cp .env.example .env
```

3. 编辑 `.env` 文件，填入你的 ChatGPT 认证信息：
```bash
# 服务器设置
SERVER_PORT=3000

# ChatGPT认证信息
CHATGPT_SESSION_TOKEN=your_session_token_here
CHATGPT_AUTHORIZATION=Bearer your_bearer_token_here

# 代理设置 (可选，推荐国内用户设置)
HTTP_PROXY=http://127.0.0.1:10809

# 可选设置
MAX_REQUESTS_PER_MINUTE=60
MAX_TOKENS_PER_MINUTE=40000
```

4. 编译并运行：
```bash
cargo build --release
./target/release/chatgpt-proxy
```

### Docker 运行

我们也提供了 Docker 支持：

```bash
# 构建镜像
docker build -t chatgpt-proxy .

# 运行容器
docker run -d -p 3000:3000 --env-file .env --name chatgpt-proxy chatgpt-proxy
```

或者使用 docker-compose：

```bash
docker-compose up -d
```

### 获取 ChatGPT 认证信息

1. 在浏览器中访问 `https://chat.openai.com/`
2. 打开开发者工具 (F12)
3. 获取 session token:
   - 切换到 Application > Cookies
   - 找到 `__Secure-next-auth.session-token` 的值
4. 获取 Authorization token:
   - 切换到 Network 标签
   - 发送一条消息给 ChatGPT
   - 在请求中找到 `Authorization` 头部的值
   - 复制完整值 (包括 `Bearer ` 前缀)

## 📡 API 使用示例

### 发送聊天请求

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [
      { "role": "system", "content": "你是一个有用的助手。" },
      { "role": "user", "content": "你好！" }
    ]
  }'
```

## 🔧 配置选项

| 环境变量 | 描述 | 默认值 |
|----------|------|--------|
| SERVER_PORT | 服务器监听端口 | 3000 |
| CHATGPT_SESSION_TOKEN | ChatGPT 会话令牌 | 无 (必填) |
| CHATGPT_AUTHORIZATION | ChatGPT 授权令牌 | 无 (必填) |
| HTTP_PROXY | HTTP 代理地址 | 无 (可选) |
| HTTPS_PROXY | HTTPS 代理地址 | 无 (可选) |
| CF_CLEARANCE | Cloudflare 验证 Cookie | 无 (可选) |
| MAX_REQUESTS_PER_MINUTE | 每分钟最大请求数 | 60 |
| MAX_TOKENS_PER_MINUTE | 每分钟最大 token 数 | 40000 |

## 🛠️ 高级使用

### 模型映射

本项目支持所有最新的 ChatGPT 模型：

- `gpt-3.5-turbo` → 传统 GPT-3.5
- `gpt-4` → 传统 GPT-4
- `gpt-4o` → 适用于大多数问题
- `gpt-4o-mini` → 更快地回答大多数问题
- `gpt-4.5` / `gpt-4.5-preview` → 研究预览版，擅长写作和构思
- `o1` → 使用高级推理
- `o1-pro` → 擅长模糊逻辑推理
- `o3-mini` → 快速进行高级推理
- `o3-mini-high` → 擅长编码和逻辑

### 代理使用

如果你在国内或其他无法直接访问 OpenAI 服务的地区，可以：

1. 在 `.env` 文件中设置代理
2. 或者在环境变量中设置 `HTTP_PROXY` 和 `HTTPS_PROXY`
3. 软件也会自动尝试常见的本地代理端口（如 10809、7890 等）

### 部署建议

推荐将服务部署在能够直接访问 OpenAI 服务的 VPS 上，这样可以避免本地网络问题和 Cloudflare 限制。

## ⚠️ 注意事项

- 此项目仅供学习和研究使用
- 请遵守 OpenAI 的服务条款
- 过度使用可能导致你的 ChatGPT 账号被限制

## 📄 许可证

MIT