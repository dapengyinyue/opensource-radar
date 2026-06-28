# 开源雷达 Open Source Radar

多源采集 GitHub / HackerNews 等渠道的开源项目，归一去重入库，提供 Web 榜单与详情展示。

Phase-1 MVP：采集 → 存储 → 展示 闭环。

## 架构

- `backend/`：Rust cargo workspace，分四层 crate
  - `domain`：纯逻辑（模型、SourceAdapter trait、归一去重）
  - `storage`：sqlx 持久层
  - `collector`：采集器 + 调度
  - `server`：axum HTTP 服务
- `frontend/`：React + Vite + TypeScript

## 本地启动

前置：Rust 工具链、PostgreSQL、pnpm。

```bash
# 1. 起 PG（本机 brew 装的 postgresql@16，或 docker compose up -d postgres）
brew services start postgresql@16
createdb openradar
createdb openradar_test

# 2. 配置
cp .env.example .env   # 填 GITHUB_TOKENS / DATABASE_URL / ADMIN_TOKEN

# 3. 后端（启动自动 migrate + axum 服务）
cargo run -p server

# 4. 手动触发采集
curl -X POST localhost:8080/api/v1/admin/collect/github -H "X-Admin-Token: $ADMIN_TOKEN"

# 5. 看榜单
curl 'localhost:8080/api/v1/projects?sort=hottest&per_page=20' | jq

# 6. 前端
cd frontend && pnpm install && pnpm dev   # http://localhost:5173
```

## 部署到 Linux（Docker Compose）

一键起 postgres + backend + frontend(nginx) 三容器，详见 [DEPLOY.md](./DEPLOY.md)。

```bash
cp .env.example .env   # 改 POSTGRES_PASSWORD / ADMIN_TOKEN / GITHUB_TOKENS
docker compose up -d --build
```

