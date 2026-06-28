# syntax=docker/dockerfile:1
# 后端多阶段构建：cargo-chef 做依赖缓存层，runtime 用 debian-slim。

FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# 限制编译并行度，低内存机器避免 OOM；高内存机器可 --build-arg CARGO_BUILD_JOBS=N 覆盖
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
COPY --from=planner /app/recipe.json recipe.json
# 先构建依赖（命中缓存层）
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin openradar

FROM debian:bookworm-slim AS runtime
# rustls 自带 webpki-roots，但保险起见装 ca-certificates；tzdata 供 chrono 本地化
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/openradar /usr/local/bin/openradar
EXPOSE 8080
CMD ["openradar"]
