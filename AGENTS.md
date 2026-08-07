# Enterprise Architecture Platform

企业管理架构平台：管理业务能力（Capability）、业务流程（Process）、价值流（Value Stream）及其关联关系。提供 GraphQL API + OpenAPI 文档 + JWT 认证。

## 技术栈

- **语言**: Rust 2021 edition, rust-version 1.75
- **Web**: axum 0.8 + tower-http
- **ORM**: sea-orm 2.0 (SQLite, sqlx-sqlite)
- **GraphQL**: async-graphql 7.0 + seaography
- **API 文档**: utoipa 5 + utoipa-swagger-ui
- **认证**: jsonwebtoken + argon2 + RSA
- **数据库**: SQLite + litestream 复制

## 构建与测试

```bash
cargo build          # 构建
cargo test           # 运行测试
cargo clippy         # lint
cargo run            # 启动服务 (默认 0.0.0.0:8080)
```

## 仓库结构

| 路径 | 职责 |
|------|------|
| `crates/shared-common` | 共享类型：枚举、错误、分页、值对象 |
| `crates/user-management` | 用户管理：认证、OAuth、令牌 |
| `crates/business-architecture` | 业务架构：能力、流程、价值流 |
| `crates/domain` | 领域模型（当前为空壳，预留 DDD 核心） |
| `crates/server` | HTTP 服务：路由、中间件、GraphQL、AI 后端 |
| `migration/` | sea-orm 迁移脚本 |
| `config/` | 配置文件 (default.toml, local.toml) |

## DDD 分层约定

`user-management` 和 `business-architecture` crate 内部按 DDD 分层：

```
crate/src/
├── domain/           # 领域模型（实体、值对象、领域事件）
├── application/      # 应用服务（用例编排、DTO 转换）
├── infrastructure/   # 基础设施（持久化、HTTP 客户端）
└── lib.rs            # 对外导出
```

**规则**：domain 不依赖 application/infrastructure；application 依赖 domain；infrastructure 依赖 domain + application。

## 配置

- `config/default.toml`：默认配置（server、database、jwt、oauth、llm）
- `config/local.toml`：本地开发覆盖
- `APP_ENV` 环境变量控制环境，未设置时默认 `"production"`（安全优先）

## 数据库迁移

迁移文件在 `migration/src/m20250101_*`。新增迁移需按序号递增。涉及数据库变更时需人工审查风险（DROP/ALTER 等）。

## GraphQL

Schema 由 `async-graphql` + `seaography` 自动生成。实体需 derive `Entity` + `Model` + GraphQL 类型。自定义 mutation 在各 crate 的 application 层实现。

## 关键约定

- migration 文件按 `m{YYYYMMDD}_{seq}_{description}.rs` 命名
- 错误处理使用 `thiserror` 定义领域错误，向上传播为 `shared-common::error::AppError`
- 分页统一使用 `shared-common::pagination::Pagination`
- AI 后端 (`crates/server/src/ai/`) 支持 OpenAI 兼容接口
