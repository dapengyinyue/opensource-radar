# 部署到 Ubuntu 24.04（Docker Compose）

三个容器一键起：`postgres` + `backend`(Rust/axum) + `frontend`(nginx 托管静态 + 反代 `/api`)。

## 1. 装 Docker

```bash
# 官方脚本安装 Docker Engine + Compose 插件
curl -fsSL https://get.docker.com | sudo sh

# 把当前用户加入 docker 组（免 sudo），重新登录生效
sudo usermod -aG docker $USER
newgrp docker

docker --version && docker compose version
```

## 2. 拉代码 + 配环境

```bash
git clone git@github.com:dapengyinyue/opensource-radar.git
cd opensource-radar
cp .env.example .env
```

编辑 `.env`，至少改这两项：

- `POSTGRES_PASSWORD` —— 改成强密码
- `ADMIN_TOKEN` —— 手动触发采集用
- `GITHUB_TOKENS` —— 填一个 GitHub classic token（[这里生成](https://github.com/settings/tokens)，无需勾选 scope，public search 够用）；不填则匿名采集，限流严格

## 3. 构建并启动

```bash
docker compose up -d --build
```

首次构建后端 Rust 会比较久（10+ 分钟，编译依赖）；后续改代码重建会命中 cargo-chef 缓存层，快很多。

查看状态：

```bash
docker compose ps
docker compose logs -f backend    # 看后端日志
```

## 4. 验证

```bash
# 前端页面（默认映射 8080 端口）
curl -I http://localhost:8080/

# API 健康
curl http://localhost:8080/api/v1/health

# 手动触发采集
curl -X POST http://localhost:8080/api/v1/admin/collect/github \
  -H "X-Admin-Token: $(grep ADMIN_TOKEN .env | cut -d= -f2)"
curl -X POST http://localhost:8080/api/v1/admin/collect/hackernews \
  -H "X-Admin-Token: $(grep ADMIN_TOKEN .env | cut -d= -f2)"

# 看榜单
curl 'http://localhost:8080/api/v1/projects?sort=stars&per_page=5' | jq
```

浏览器打开 `http://<服务器IP>:8080/` 即可。

## 5. 常用运维

```bash
docker compose up -d --build       # 改代码后重建
docker compose restart backend     # 仅重启后端
docker compose logs -f --tail=100  # 跟踪日志
docker compose down                # 停止（数据卷保留）
docker compose down -v             # 停止并删除数据库数据（慎用）
```

数据库备份：

```bash
docker compose exec postgres pg_dump -U openradar openradar > backup.sql
```

## 6. 上 HTTPS（可选，生产建议）

在前端容器前再挂一层宿主机 nginx/Caddy 反代并终结 TLS，或直接把 `WEB_PORT` 改成 `80` 配合 Caddy 自动证书。最简方案：

```bash
# 宿主机装 Caddy，Caddyfile 一行自动 HTTPS
sudo apt install caddy
# /etc/caddy/Caddyfile:
#   your-domain.com {
#       reverse_proxy localhost:8080
#   }
sudo systemctl reload caddy
```

## 端口与服务拓扑

```
宿主机:8080 ──► frontend(nginx:80)
                    │ 静态文件 dist/
                    └─/api/ 反代 ──► backend:8080 (axum, 仅内部网络)
                                          │
                                          └─► postgres:5432 (仅内部网络)
```

backend 与 postgres 不对宿主机暴露端口，只有 frontend 的 80 映射到宿主机 `WEB_PORT`。
