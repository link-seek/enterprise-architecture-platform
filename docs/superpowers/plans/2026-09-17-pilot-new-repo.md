# 试点期（新仓试点）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 xieyucheng.top 上定义一条价值流并运行一次，无人手工干预建出试点消费仓且走完 `issue→img→img→pro` 全程 2 次（对应 #562 试点期方案）。

**Architecture:** EAP 只做编排（价值流运行实例 + 生成器 mutation + 链骨架模板），所有执行流程复用 L1 已有 reusable workflow（discuss/fix/pr-ci/deploy），试点仓以版本 pin 引用 L1，密钥全程后端持有。

**Tech Stack:** Rust/axum（与 EAP 同栈）+ GitHub Actions（L1 reusable）+ SQLite/sea-orm 迁移 + OSS 静态前端 + 后端机部署（与 EAP 同款）。

## Global Constraints

- L1 是唯一流程定义方：试点仓的 workflow 只许 `uses: link-seek/issue-resolver-l1/.github/workflows/*.yml@<TAG>`，禁止 `@main`（#562 决策 4，bot 已证实当前 11 处全是 `@main`）。
- 消费仓薄壳原则（L1 PRINCIPLES.md）：EAP/L1 agent 不直接改试点仓 `.github/workflows/`，生成器落仓走 PR + L1 PR CI。
- 密钥不落地前端：试点仓所需 `APP_ID/APP_PRIVATE_KEY`/镜像库/OSS/部署密钥由平台方建仓时注入，EAP 后端只存引用不回显。
- 试点用新空间干净数据，不碰 Seed dogfood（`seed_dogfood.rs`）。
- 模型统一走 OpenCode Go 付费（`DISCUSS_MODEL=muse-spark-1.3-contributor`，base `https://opencode.ai/zen/go/v1`）；试点仓复用同一套 secret 名。

---

## 今日进展对照（2026-09-17，#562 定稿后 1 天）

| #562 要求 | 现状 | 缺口 |
|---|---|---|
| 决策 3：能力= L1 `workflow_call` 接口 | ✅ L1 9 个流程全是 reusable，EAP 侧 `on-fix/on-pr-review/on-discuss/on-push` 均为薄调用 | 无 |
| 决策 4：订阅+版本 pin | ❌ L1 零 tag（只有 `l2-backup/*`），EAP 11 处 `@main` | Phase 0 补 |
| 试点 6 节之脚手架/模板 | ❌ EAP 无 `templates/` 目录 | Phase 1 建 |
| 试点 6 节之生成器 mutation | ❌ 不存在 | Phase 2 建 |
| 试点 6 节之运行实例实体 | ❌ `value_streams` 只有定义态，`LifecycleStatus` 无运行态 | Phase 3 建模 |
| 验收：无人干预 2 次 + PR CI 绿 + 回滚演练 | 今日已证明同款链路可行：#652→#660→镜像 `20260917040635-0cb6d20c`（04:11） | 在试点仓复现 |
| 建仓三件套（App 权限/Secret 落点/模板存放） | ❌ bot 判 blocked，至今未补 | Phase 0/1 补，需 org admin 配合 |
| 今日新增原则：验证与门禁解耦 | ✅ 已记 L1 PRINCIPLES.md；pipeline-test 已删 | 试点验收跑 2 次全程时不要复用已删 harness，用本计划 Phase 4 步骤 |

---

### Task 1: L1 首个版本 tag + EAP pin（版本纪律）

**Files:**
- Modify: `eap-github/.github/workflows/on-fix.yml:25,37,78`（`@main`→`@v1.0.0-pilot`，共 11 处，同文件 `on-pr-review.yml:14`、`on-pr.yml`、`on-push.yml:18`、`on-deploy.yml` 等一并改）
- Create: L1 tag `v1.0.0-pilot`（含今日 main 头全部内容：Zen 迁移 + ci 瘦身 + ghcr 登录）

**Interfaces:**
- Consumes: L1 main（`63aae56` 及之后，需先确认无未合入分支）
- Produces: 不可变引用 `link-seek/issue-resolver-l1/.github/workflows/*.yml@v1.0.0-pilot`，供 Task 4 试点仓链骨架使用

- [ ] **Step 1: 确认 L1 main 可发版**
  Run: `git log --oneline origin/main -5` + `gh pr list --limit 5`（L1 仓）
  Expected: 无 OPEN 的 PR，无未合入分支
- [ ] **Step 2: 打 tag 并推送**
  Run: `git tag -a v1.0.0-pilot -m "pilot baseline: zen+slim ghcr-login" && git push origin v1.0.0-pilot`
  Expected: `gh api repos/link-seek/issue-resolver-l1/git/refs/tags/v1.0.0-pilot` 返回 200
- [ ] **Step 3: EAP 11 处 `@main`→`@v1.0.0-pilot`（EAP 薄壳，人工改）**
  Run: `grep -rn "issue-resolver-l1/.*@main" eap-github/.github/workflows/`
  Expected: 0 结果（11→0）
- [ ] **Step 4: 发 EAP 测试 PR 验证 pin 有效**
  Run: 任意小改动 PR，观察 `review-ai` 照常跑
  Expected: PR CI 全绿（证明 pin 后链路不断）
- [ ] **Step 5: Commit**（EAP 侧）
  ```bash
  git add .github/workflows/
  git commit -m "chore: pin L1 workflows to v1.0.0-pilot for pilot"
  ```

### Task 2: 试点仓脚手架模板

**Files:**
- Create: `eap-github/templates/pilot-consumer/` 下 `Dockerfile`、`docker-compose.ci.yml`、后端 `src/main.rs`（`/health`）、前端 `index.html`（Hello World）、`.github/workflows/on-fix.yml`、`on-pr.yml`、`on-push.yml`、`on-deploy.yml`（全部 `uses: ...@v1.0.0-pilot`）、`.issue-resolver.yml`
- Modify: 无

**Interfaces:**
- Consumes: Task 1 的 tag 名（模板内写死 `@v1.0.0-pilot`）
- Produces: 模板目录，Task 4 生成器按此渲染

- [ ] **Step 1: 建目录骨架并从 EAP 精简复制**
  以 `docker-compose.ci.yml`、`deploy/pod.yml` 为蓝本删减到最小（单后端 + 单静态页），禁止复制业务代码
- [ ] **Step 2: 四个薄壳 workflow 全部 pin**
  Run: `grep -rn "@main" eap-github/templates/pilot-consumer/.github/`
  Expected: 0 结果
- [ ] **Step 3: 本地 compose 拉起验证**
  Run: `docker compose -f templates/pilot-consumer/docker-compose.ci.yml up -d && curl -sf localhost:8080/health`
  Expected: HTTP 200
- [ ] **Step 4: Commit**
  ```bash
  git add eap-github/templates/pilot-consumer/
  git commit -m "feat: pilot consumer scaffold template (pinned L1 v1.0.0-pilot)"
  ```

### Task 3: 建仓三件套（需 org admin 配合，indivisible）

**Files:** 无（权限与 secret 操作，不落代码）

- [ ] **Step 1: GitHub App 追加建仓权限**：`contents:write/pull-requests:write/issues:write/actions:write`（对齐 `on-fix.yml`）+ `administration:write`（建仓）或预授权 org 模板仓，org admin 在 GitHub 后台一次配好
- [ ] **Step 2: 试点仓 secret 清单建仓时一次性注入**：`APP_ID/APP_PRIVATE_KEY`、GHCR、OSS（`oss-bucket: pilot-frontend-xyc` 新建）、后端机 SSH，EAP 后端只存引用名
- [ ] **Step 3: 验收三件套就绪**：空仓手动建一次（admin 操作），确认 App 能推、secret 全、模板渲染路径通后删空仓
  Expected: 三项 checklist 全勾方可进 Task 4（#562 bot 判 blocked 项，不可跳过）

### Task 4: 生成器（EAP mutation → 渲染 → 走 PR 落仓）

**Files:**
- Create: `eap-github/backend/crates/server/src/pilot.rs`（`provisionPilotConsumer(spaceId, templateVersion)` mutation：调 GitHub API 建仓→渲染 Task 2 模板→提初始 PR）
- Modify: `eap-github/backend/crates/server/src/graphql.rs`（注册 mutation，不碰现有 `ensure_*` 门禁逻辑）
- Test: `eap-github/backend` 新增单测：模板渲染快照测试（输入固定 inputs → 输出 chain/inputs 文件字节一致）

**Interfaces:**
- Consumes: Task 2 模板目录；Task 3 的 App 凭证（后端持有）
- Produces: 试点仓 + 初始 PR（必须过 L1 PR CI 方可合入，与 #562 §3 一致）

- [ ] **Step 1: 写渲染快照测试（先红）**
  Run: `cargo test -p eap-server pilot_render`
  Expected: FAIL（mutation 不存在）
- [ ] **Step 2: 实现 mutation 最小闭环**（建仓→渲染→push→开 PR 四步，失败即回滚删仓）
- [ ] **Step 3: 单测转绿**
  Run: `cargo test -p eap-server pilot_render`
  Expected: PASS
- [ ] **Step 4: Commit**
  ```bash
  git commit -m "feat: provisionPilotConsumer mutation (template render + PR)"
  ```

### Task 5: 运行实例数据模型

**Files:**
- Create: `eap-github/backend/migration/src/m20260917_000001_value_stream_runs.rs`（表 `value_stream_runs`：`id/value_stream_id/repo_url/chain_path/status/pinned_versions/snapshot_json`，`status` 新枚举 `defined/provisioning/live/archived`，不复用 `LifecycleStatus`）
- Modify: `eap-github/backend/crates/business-architecture/.../entities/`（sea-orm 实体）、GraphQL 查询（运行实例状态页只读）
- Test: migration 正反向测试（`cargo test -p migration`）

**Interfaces:**
- Consumes: Task 4 产出的 repo_url/chain_path（provisioning→live 状态流转由生成器回写）
- Produces: 运行实例读写，Task 6 验收计数依据（"2 次全程"以 `live` 状态行数为准，不是看镜像 tag）

- [ ] **Step 1: 写迁移文件 + 正反向测试**
- [ ] **Step 2: Run: `cargo test -p migration` Expected: PASS**
- [ ] **Step 3: GraphQL 只读查询 + 前端状态页最小展示（复用现有 spaces 布局，不新开路由体系）**
- [ ] **Step 4: Commit**

### Task 6: 验收（2 次全程 + 回滚演练 + 反脆弱两条）

**Files:** 无（执行既有链路，计数落 `value_stream_runs`）

- [ ] **Step 1: xieyucheng.top 定义价值流→运行→无人干预走完 `issue→img→img→pro`，第 1 次**
  Expected: `value_stream_runs` 新增 1 行 `live`，镜像 tag 可查
- [ ] **Step 2: 同上，第 2 次**（新运行实例，验证可重复）
- [ ] **Step 3: 模拟回滚演练**：revert 生成器 PR + 重新部署上一镜像，双动作缺一不可
  Expected: 服务回到上一版本且健康检查 200
- [ ] **Step 4（反脆弱①）: L1 发版一次（`v1.0.1`）后试点仍可复现**：试点仓保持 pin 旧版可跑，升级 PR 走 L1 PR CI
  Expected: pin 有效性得证
- [ ] **Step 5（反脆弱②）: Secret 轮换一次后链路仍通**：轮换试点仓 `APP_PRIVATE_KEY`，重跑日常迭代
  Expected: 无手工 hardcode 得证
- [ ] **Step 6: 达标评审**：六项全勾才谈自举期（EAP 自身纳管），否则回 Task 1–5 修

---

## Self-Review

- **Spec coverage:** 8 决策→决策 1/2/5/6 为背景约束（无需任务）；决策 3→Task 2/4（接口复用）；决策 4→Task 1（pin）+ Task 5 订阅字段（`pinned_versions` 列已含；独立订阅表延后，因试点单仓无跨空间订阅场景，YAGNI）；决策 7/8→任务顺序与模板形态（Task 2/4）。6 节→目标/脚手架/生成器/模型/验收/范围外逐一映射，deferred 项未建任务 ✅
- **Placeholder scan:** 无 TBD/TODO；每步含文件路径与命令及预期输出 ✅
- **Type consistency:** tag 名 `v1.0.0-pilot` 全文统一；`value_stream_runs` 字段在 Task 4/5/6 一致 ✅
