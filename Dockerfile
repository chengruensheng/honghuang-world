# 洪荒·世界 Docker 镜像（rust 1.78 多阶段构建）
#
# 依据：§B.1.1 投产基础（Dockerfile + docker-compose）
# 用途：CI + 生产部署统一镜像；项目所有 crate + bin 单一入口

# ---- 阶段 1：构建 ----
FROM rust:1.78-slim AS builder

# 系统依赖（sccache + mold 加速 + pkg-config for ring）
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates sccache mold \
 && rm -rf /var/lib/apt/lists/*

# 加速 cargo：sccache + 共享 target 目录 + lint 不在 docker 跑
ENV RUSTC_WRAPPER=sccache \
    SCCACHE_DIR=/tmp/sccache \
    CARGO_INCREMENTAL=1 \
    CARGO_TERM_COLOR=always

WORKDIR /build

# 先复制 Cargo.toml + lock（缓存依赖）
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo

# 预热依赖缓存（避免 COPY src 改变后重编译所有依赖）
RUN mkdir -p src && echo "fn main(){}" > src/main.rs \
 && cargo build --release --bin 监控 || true

# 复制源码 + 构建
COPY . .
RUN cargo build --release \
 && sccache --show-stats || true

# ---- 阶段 2：运行时 ----
FROM debian:bookworm-slim AS runtime

# 运行时依赖（ca-certificates + libssl for rustls）
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 tini \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd -g 1001 world \
 && useradd -u 1001 -g world -m -s /bin/bash world

# 从 builder 复制构建产物（保留目录结构，按需 COPY）
WORKDIR /app
COPY --from=builder /build/target/release/ /app/bin/
COPY --from=builder /build/道果树/构建物-域/release/ /app/bin/

# 复制 .上下文 配置园资产（不写状态 — 状态由挂载卷管理）
COPY .上下文/ .上下文/ 2>/dev/null || true

# 数据目录（运行时挂载卷）
RUN mkdir -p /data/.上下文/{状态,观测,格位,对话} \
 && chown -R world:world /data /app

USER world
WORKDIR /app

# 健康检查（jiankong_fu 删了 — 占位，等监控界面 2.0 重新设计后填实际地址）
# 现阶段用世界进程存活探针（ENTRYPOINT 不退即 healthy）
# HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
#   CMD curl -f http://127.0.0.1:8080/healthz || exit 1

# tini 收僵尸进程 + 信号转发
ENTRYPOINT ["/usr/bin/tini", "--"]

# 默认入口：世界运行守护（守护循环 + 鸿钧对话 + 任务线 + 巡世扫描）
# 注：项目无 main.rs — 这是 B.1.1 占位；B.1.4 graceful shutdown 阶段补实际入口
# CMD ["/app/bin/世界运行守护"]
CMD ["/bin/sh", "-c", "echo 'B.1.1 Dockerfile ready — 监控界面 2.0 待重新设计' && tail -f /dev/null"]

# 元数据
LABEL org.opencontainers.image.title="洪荒·世界" \
      org.opencontainers.image.description="多智能体身+脑融合 + 世界自进化" \
      org.opencontainers.image.version="v0.1.0" \
      org.opencontainers.image.licenses="MIT"
