# pilot-consumer 模板（试点期脚手架）

试点消费仓最小骨架。所有执行流程 pin L1 `v1.0.0-pilot`，禁止 `@main`。

## 结构

- `backend/`：零依赖 Rust `/health`（`{"status":"ok"}`），`cargo test` 自带 2 个单测
- `frontend/`：纯静态 Hello World + nginx（`/health`、`/api/` 反代 backend）
- `docker-compose.ci.yml`：CI/E2E 用编排（backend 8080 + frontend 80）
- `.github/workflows/`：四个薄壳（on-fix/on-pr/on-push/on-deploy），全部 `uses: ...@v1.0.9-pilot`
- `.issue-resolver.yml`：触发 `fix-me` / `@oh`，测试命令 `cargo test`

## 生成器占位（`__PILOT_*`，渲染时替换）

| 占位 | 含义 |
|---|---|
| `__PILOT_REPO__` | 试点仓名（如 `pilot-demo`），用于 ghcr-image 与 deploy 并发组 |
| `__PILOT_RUNNER__` | 部署 runner（如试点复用 EAP 后端机则填对应 label） |
| `__PILOT_FRONTEND_URL__` / `__PILOT_API_URL__` | 试点前后端公网地址 |

## 本地验证

```bash
docker compose -f templates/pilot-consumer/docker-compose.ci.yml up -d --build
curl -sf localhost:8080/health && curl -sf localhost/health
cargo test --manifest-path templates/pilot-consumer/backend/Cargo.toml
```
