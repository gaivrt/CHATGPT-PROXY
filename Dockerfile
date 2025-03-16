FROM rust:slim-bullseye as builder

# 创建工作目录
WORKDIR /app

# 复制项目文件
COPY . .

# 构建项目
RUN cargo build --release

# 使用更小的运行时镜像
FROM debian:bullseye-slim

# 安装必要依赖
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# 创建不需要root权限的用户
RUN useradd -m appuser

# 创建工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/chatgpt-proxy /app/chatgpt-proxy

# 复制.env.example文件作为参考
COPY --from=builder /app/.env.example /app/.env.example

# 设置运行权限
RUN chown -R appuser:appuser /app
USER appuser

# 设置默认端口
ENV SERVER_PORT=3000

# 暴露默认端口
EXPOSE 3000

# 运行应用
CMD ["/app/chatgpt-proxy"]