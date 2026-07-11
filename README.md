# 开源雷达 Open Source Radar

开源雷达是一个用于发现和观察开源项目的 Web 应用：从 GitHub / HackerNews 采集项目与讨论，归一去重后入库，提供榜单、搜索、筛选、详情与趋势展示。

当前目标不是做一个「又一个 GitHub Trending」，而是跑通一条可扩展的数据链路：

> 多源采集 → 归一去重 → 指标快照 → 可搜索/可筛选/可解释的发现界面

## 当前能力

### 数据采集

- GitHub Search API：采集高星开源仓库
- HackerNews Algolia API：采集热门 HN 故事
- 多源归一去重：HN 链接指向 GitHub repo 时，与 GitHub 源项目合并为同一个 `project`
- 定时后台采集：GitHub / HN 独立周期运行
- 手动触发采集：`POST /api/v1/admin/collect/:source`
- 指标快照：每次采集写入 `project_snapshots`，用于趋势展示

### Web 交互

- 首页榜单：综合热度 / Stars / 最近活跃 / HN 热度排序
- 全局搜索：按项目名、`owner/repo`、描述搜索
- 筛选：语言、Topic、来源、时间窗口
- 分页：URL 同步 `page`，榜单数据不再只停留在前 50 条
- 数据时效：展示 GitHub / HN 上次采集时间与项目数量
- 详情页：
  - 按来源分组展示 GitHub 指标与 HackerNews 热度
  - 展示 HN 讨论外链、作者、发布时间、分数、评论数
  - 指标趋势图按 Stars / HN Points 分线展示，避免跨源数值混线
- URL 状态保持：筛选后进入详情再返回，保留原筛选状态

## 技术栈

### Backend

- Rust
- axum：HTTP API
- tokio：异步运行时与后台调度
- sqlx：PostgreSQL 持久层
- reqwest：GitHub / HN API 客户端
- tracing：结构化日志

Backend 是 Cargo workspace：

```text
backend/
  crates/
    domain/     # 纯领域逻辑：模型、SourceAdapter trait、归一去重
    storage/    # sqlx repository：project/raw/snapshot/facet 查询
    collector/  # GitHub/HN adapter、限流、调度、token 轮换
    server/     # axum HTTP 服务
```

### Frontend

- React
- Vite
- TypeScript
- Tailwind CSS
- TanStack Query
- React Router

### Infra

- PostgreSQL 16
- Docker Compose
- nginx 容器托管前端静态文件，并反代 `/api` 到 backend

## 目录结构

```text
.
├── backend/              # Rust workspace
│   ├── crates/
│   ├── migrations/
│   └── Dockerfile
├── frontend/             # React/Vite 前端
│   ├── src/
│   ├── Dockerfile
│   └── nginx.conf
├── docker-compose.yml    # postgres + backend + frontend
├── DEPLOY.md             # Ubuntu / Docker Compose 部署说明
└── .env.example
```

## 本地开发

前置：Rust 工具链、PostgreSQL、pnpm。

### 1. 准备数据库

```bash
brew services start postgresql@16
createdb openradar
createdb openradar_test
```

Linux 环境也可以直接使用系统 PostgreSQL，或用 Docker 起一个本地 PG。

### 2. 配置环境变量

```bash
cp .env.example .env
```

至少需要关注：

```bash
DATABASE_URL=postgres://localhost:5432/openradar
DATABASE_URL_TEST=postgres://localhost:5432/openradar_test
GITHUB_TOKENS=       # 可空；建议填 GitHub token 提高限流额度
ADMIN_TOKEN=change-me
```

### 3. 启动后端

```bash
cd backend
cargo run -p server
```

后端启动时会自动执行 migration，并启动后台采集器。

健康检查：

```bash
curl http://localhost:8080/api/v1/health
```

### 4. 启动前端

```bash
cd frontend
pnpm install
pnpm dev
```

浏览器打开：

```text
http://localhost:5173
```

Vite 会把 `/api` 代理到 `http://localhost:8080`。

## Docker Compose 部署

一键起 `postgres + backend + frontend(nginx)` 三个容器：

```bash
cp .env.example .env
# 编辑 .env：至少改 POSTGRES_PASSWORD / ADMIN_TOKEN / GITHUB_TOKENS

docker compose up -d --build
```

默认访问：

```text
http://localhost:8080
```

生产 / Ubuntu 24.04 详细步骤见 [DEPLOY.md](./DEPLOY.md)。

## 常用 API

```bash
# 健康检查
GET /api/v1/health

# 项目榜单
GET /api/v1/projects?page=1&per_page=50&sort=hottest

# 搜索
GET /api/v1/projects?q=awesome

# 筛选
GET /api/v1/projects?language=Rust&topic=llm&source=github&since=30d

# 详情
GET /api/v1/projects/:id

# 指标快照
GET /api/v1/projects/:id/snapshots

# 来源明细（GitHub repo + HN stories）
GET /api/v1/projects/:id/sources

# facets
GET /api/v1/languages
GET /api/v1/topics

# 采集状态
GET /api/v1/sources/status

# 手动触发采集
POST /api/v1/admin/collect/github
POST /api/v1/admin/collect/hackernews
```

手动触发采集需要 Header：

```bash
-H "X-Admin-Token: $ADMIN_TOKEN"
```

## 测试

Backend：

```bash
cd backend
DATABASE_URL_TEST=postgres://localhost:5432/openradar_test cargo test --workspace
cargo clippy --workspace --all-targets
```

Frontend：

```bash
cd frontend
pnpm typecheck
pnpm build
```

## 数据模型要点

- `projects`：归一后的项目主表，一个真实项目一行
- `raw_github_repos`：GitHub 源原始数据
- `raw_hn_stories`：HN 源原始数据，保存 HN 讨论链接
- `project_snapshots`：每次采集的指标快照

核心去重键：

- GitHub repo：`gh:owner/repo`
- 非 GitHub 外链：`url:host/path`
- 无外链 HN 故事：`hn:object_id`

## 后续方向

- Rising / 上升最快：基于 `project_snapshots` 计算真实增量
- 订阅监控：订阅 repo/topic/language，推送 release / star / HN 热度变化
- 更多数据源：Reddit、V2EX、ProductHunt 等
- 更强搜索：`pg_trgm` / 全文索引 / 向量语义搜索
- RSS / 周报：按 topic 自动生成开源项目周报

## License

MIT
