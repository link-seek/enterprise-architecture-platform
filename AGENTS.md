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


## 架构分域（业务/应用）与跨域映射（2026-08 补充）

- **前端信息架构**：空间架构区侧边栏（`frontend/src/views/architectures/layout.tsx`）按
  「业务架构 / 应用架构」两组视觉分组，路由保持扁平（`/spaces/:spaceId/architectures/*`）。
  架构区 index 与登录后跳转目标为 `overview`（架构总览页），登录跳转见 `frontend/src/views/login.tsx`。
- **跨域映射查询**（`backend/crates/server/src/graphql.rs`）：
  - 逐实体：`capabilityRealizationsByCapability`、`processReferencesByBusinessProcess`、`processReferencesByApplicationProcess`
  - 按空间聚合（总览页/映射页用于消除 N+1）：`capabilityRealizationsBySpace`、`processReferencesBySpace`
  - 注意 `ProcessReferences` GraphQL 类型**没有 `id` 字段**，查询只选 `applicationProcessId`/`businessProcessId`；
    `capability_realizations.process_type` 为 `business_process | application_process`。
  - 本地默认限流 4 req/s（burst 25），页面若发起 10+ 并行查询会触发 429；CI 用 100 req/s。
- **Playwright 注意**：`getByRole(name)` 默认是**子串匹配**；总览页入口卡片是整卡 Link，
  accessible name 含实体名（如 `价值流 1 …`），会与侧边栏链接子串冲突，故侧边栏断言需 `exact: true`；
  分组标题「业务架构/应用架构」同时出现在侧边栏 h3 与总览页 h2，需按 `navigation`/`main` 作用域区分。
- 登录默认落点改为 overview 后，value-stream 类测试需显式 `goto SPACE_BASE + "/value-streams"`。

## E2E 测试编写规范

### 命名与隔离

- **所有 Create/Delete/Archive 测试必须用 `Date.now()` 或 `crypto.randomUUID()` 生成唯一名称**，禁止硬编码如 `待删除能力`。
  原因：硬编码名称在残留数据时会导致 `not.toBeVisible()` 误判通过（上次同名项已被删除，断言照过）。
- Create 测试同理，用唯一名称避免与已有数据冲突。

### 删除验证

- **Delete/Archive 测试必须在删除操作后刷新页面，再次断言数据消失**：
  ```ts
  // 点击删除确认
  await dialog.getByRole('button', { name: '删除' }).click();
  await expect(dialog).not.toBeVisible();

  // 刷新页面验证后端确实删除了，而非仅前端乐观更新
  await page.reload();
  await expect(page.getByText(name, { exact: true })).not.toBeVisible();
  ```
  原因：仅验证 UI 行消失无法区分「后端真删除」和「前端乐观更新但后端失败」。
  #403 bug 就是删除对话框静默吞掉 GraphQL 错误的典型案例。

### GraphQL 错误检测

- 所有 E2E 测试默认使用 `graphql-aware` fixture（`import { test } from '../helpers/graphql-aware'`），
  该 fixture 自动检测 GraphQL errors 并让测试失败。
- **禁止** `import { test } from '@playwright/test'` 绕过 GraphQL 错误检测。

### 标签

- 页面加载/导航类轻量测试：`@smoke`
- CRUD/权限/成员管理等功能测试：`@regression`
- **Create + Delete 必须同时有 `@smoke` 和 `@regression`**，确保 Smoke Test 覆盖 CRUD 完整流程。
- 标签格式：`{ tag: ['@smoke', '@regression'] }`（数组，不覆盖其他标签）

### 断言

- 用 `exact: true` 匹配特定文本，避免子串匹配误判。
- 用 `page.locator('tr').filter({ hasText: name })` 精确定位行，而非全局 `page.getByText()`。
- 删除后断言用唯一名称 + `exact: true`，避免匹配到其他含相同子串的文本。
