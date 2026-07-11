# 日报推送（Server酱）设计

日期：2026-07-11
状态：已确认，待实现
分支：feat/digest-push

## 目标

每天 09:00 把「今日新发现」的开源项目 TOP10 推送到用户微信（经 Server酱 / sct.ftqq.com）。让雷达从「被动展示」升级为「主动触达」。

## 范围（MVP）

- 推送通道：仅 Server酱
- 推送内容：近 24h 内 `first_seen_at` 的新项目，按综合热度排序 TOP N
- 推送时刻：每天固定 `DIGEST_HOUR`（默认 09:00）
- 手动触发：admin 端点立即发一次（验证用）
- 多通道扩展点：`Notifier` trait 预留

## 不做（YAGNI）

- 飞书/钉钉等其他通道（trait 预留，不实现）
- 订阅/自定义筛选推送（留后续）
- 上升最快（stars 增量）日报（需先实现增量计算，留后续）
- 推送失败重试（单次发送，失败记日志，下个周期再来）
- 去重/防重发（每天一个自然周期，不重复）

## 架构

### 数据层（storage）

`ProjectFilter` 新增字段：

```rust
/// 首次发现时间下限（日报「今日新发现」用）
pub first_seen_since: Option<DateTime<Utc>>,
```

`list` / `count` 的 SQL WHERE 各加一行：

```sql
AND ($n::timestamptz IS NULL OR first_seen_at >= $n)
```

这让「新发现」能力同时服务前端筛选（将来可加 `?first_seen=24h`），不是日报专属。

### 通知抽象（domain）

`domain::notifier`：

```rust
#[async_trait::async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, title: &str, desp: &str) -> Result<(), NotifyError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("http error: {0}")]
    Http(String),
    #[error("serverchan api error: code={code} message={message}")]
    Api { code: i64, message: String },
    #[error("not configured")]
    NotConfigured,
}
```

domain 不依赖 reqwest（Http 错误由 adapter 转成字符串）。

### ServerChan 通知器（collector::notifier）

```rust
pub struct ServerChanNotifier {
    client: reqwest::Client,
    sendkey: String,
    base_url: String,  // 测试可注入，默认 https://sct.ftqq.com
}
```

`send(title, desp)`：POST `{base_url}/{sendkey}.send`，form 表单 `title` + `desp`。
- 响应 JSON `{"code": 0}` 成功；非 0 报 `Api` 错误
- HTTP 失败报 `Http` 错误

### 日报生成（collector::digest）

```rust
pub struct DigestReport {
    pub title: String,    // "开源雷达日报 · 7月11日"
    pub markdown: String, // ServerChan desp（markdown）
    pub count: usize,
}

/// 生成今日新发现日报。无新项目返回 Ok(None)（当天不发空报）。
pub async fn generate_digest(pool: &PgPool, top_n: i64) -> Result<Option<DigestReport>>;

/// 计算到下个 digest_hour 的延迟（纯函数，可测）。
pub fn next_digest_delay(now: DateTime<Utc>, hour: u32) -> Duration;
```

`generate_digest` 实现：
1. `list(pool, filter { first_seen_since: now-24h, sort: Hottest, per_page: top_n })`
2. 空则返回 `None`
3. 拼 markdown：

```markdown
## 今日新发现 TOP 10

1. **[tokio-rs/tokio](https://github.com/tokio-rs/tokio)** ⭐27.0k · 🟧120
   async runtime
2. ...
```

`next_digest_delay` 逻辑：
- 算今天 `hour:00` 的时刻
- 若 `now < 今天 hour:00` -> 延迟到今天 hour:00
- 否则 -> 延迟到明天 hour:00
- 返回 `Duration`

### 调度（collector::digest_scheduler）

```rust
pub struct DigestScheduler {
    pool: PgPool,
    notifier: Arc<dyn Notifier>,
    hour: u32,
    top_n: i64,
    cancel: CancellationToken,
}

impl DigestScheduler {
    /// spawn 定时任务：sleep 到 09:00 -> 发日报 -> 每 24h 一次
    pub fn start(&self) -> JoinHandle<()>;
}
```

任务循环：
```
let delay = next_digest_delay(Utc::now(), hour);
sleep(delay);
loop {
    match generate_digest(&pool, top_n).await {
        Ok(Some(report)) => if let Err(e) = notifier.send(&report.title, &report.markdown).await {
            warn!(error=%e, "digest push failed");
        },
        Ok(None) => info!("no new projects today, skip digest"),
        Err(e) => warn!(error=%e, "generate digest failed"),
    }
    select! { _ = sleep(24h) => {}, _ = cancel.cancelled() => break; }
}
```

### 配置（server::config）

`Settings` 新增：

```rust
pub serverchan_sendkey: Option<String>,  // env SERVERCHAN_SENDKEY
pub digest_hour: u32,                    // env DIGEST_HOUR，默认 9
pub digest_top_n: i64,                   // env DIGEST_TOP_N，默认 10
```

`serverchan_sendkey` 为空 -> 不启动日报调度（日志一条 info）。

### 手动触发端点（server::api::digest）

`POST /api/v1/admin/digest`：
- admin token 鉴权
- fire-and-forget spawn：`generate_digest + notifier.send`
- 立即返回 202 + 报告内容预览（`{ status: "accepted", count, preview: "..." }`）
- 注意：需要 notifier 注入 AppState。若 sendkey 未配置，返回 400「推送未配置」

### 装配（server::lib.rs run）

```rust
// 构建 notifier
let notifier: Option<Arc<dyn Notifier>> = cfg.serverchan_sendkey.as_ref().map(|key| {
    Arc::new(ServerChanNotifier::new(client.clone(), key.clone())) as Arc<dyn Notifier>
});

// 启动日报调度（sendkey 为空则跳过）
if let Some(n) = &notifier {
    let sched = DigestScheduler::new(pool.clone(), n.clone(), cfg.digest_hour, cfg.digest_top_n, cancel.clone());
    sched.start();
} else {
    info!("SERVERCHAN_SENDKEY not set, digest push disabled");
}

// AppState 持有 notifier（admin 端点用）
let state = AppState { pool, collector, admin_token, notifier };
```

## 测试策略（TDD）

1. **storage**：`first_seen_since` 过滤--插一条 first_seen_at 在 24h 内、一条在外，断言只命中前者。加到 `repo_integration.rs`。
2. **next_digest_delay**（纯函数）：表驱动--`08:00 -> 1h`、`10:00 -> 23h`、`09:00 整点 -> 24h`、`hour=0` 边界。
3. **ServerChanNotifier**：wiremock mock `sct.ftqq.com/{key}.send`，断言 POST 表单 `title`/`desp` 正确；响应 `{"code":0}` 成功；响应 `{"code":40001}` 报 Api 错误。
4. **generate_digest**：testcontainers/本地 PG，插两条新项目 + 一条老项目，断言返回 Some、markdown 含新项目名、count 正确、老项目不在。
5. **API**：`POST /admin/digest` 无 token 401、有 token + sendkey 未配置 400、有 token + 配置 202 且返回 preview。

## 文件清单

```
backend/crates/domain/src/notifier.rs             # Notifier trait + NotifyError
backend/crates/domain/src/lib.rs                  # pub mod notifier
backend/crates/storage/src/repo/project.rs        # first_seen_since 字段 + SQL
backend/crates/collector/src/notifier.rs          # ServerChanNotifier
backend/crates/collector/src/digest.rs            # generate_digest + next_digest_delay
backend/crates/collector/src/digest_scheduler.rs  # DigestScheduler
backend/crates/collector/src/lib.rs               # pub mod 三个
backend/crates/server/src/config.rs               # 三个新字段
backend/crates/server/src/api/digest.rs           # admin 触发端点
backend/crates/server/src/app.rs                  # 路由 + AppState.notifier
backend/crates/server/src/lib.rs                  # 装配
backend/.env.example + /.env.example              # SERVERCHAN_SENDKEY / DIGEST_HOUR / DIGEST_TOP_N
```

## 实现顺序（每步可独立验证）

1. storage: `first_seen_since` + 测试
2. domain: Notifier trait + NotifyError
3. collector: `next_digest_delay` 纯函数 + 表驱动测试
4. collector: `ServerChanNotifier` + wiremock 测试
5. collector: `generate_digest` + 测试
6. collector: `DigestScheduler`（装配用，逻辑轻）
7. server: config 三字段 + .env.example
8. server: `POST /admin/digest` 端点 + 路由 + AppState + 测试
9. server: run() 装配 notifier + scheduler
10. 全量测试 + fmt + clippy
11. 部署 107 + 真实推送验证

## 验证（部署后）

```bash
# 107 上配 sendkey
echo 'SERVERCHAN_SENDKEY=SCT你的key' >> ~/opensource-radar/.env

# 重建
cd ~/opensource-radar && git pull && docker compose up -d --build

# 手动触发一次日报（不等 09:00）
curl -X POST http://localhost:8080/api/v1/admin/digest -H "X-Admin-Token: $ADMIN_TOKEN"
# 预期：202 + { count, preview }，微信收到推送
```

成功标准：微信收到「开源雷达日报 · 7月11日」推送，内容为今日新发现 TOP10 markdown。
