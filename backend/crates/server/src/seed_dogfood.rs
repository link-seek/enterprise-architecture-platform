//! Dogfood seed: models the EAP platform's own development flow as a
//! two-layer (business + application) architecture inside the seeded test
//! space. Implements the P1 (business architecture) and P5 (application
//! architecture + `realizes` mappings) work items from Discussion #336.
//!
//! All entities use fixed UUIDs so the seed is fully idempotent: re-running
//! on an already-seeded database is a no-op, and child rows reference their
//! parents by the known ID without a lookup round-trip.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, PrimaryKeyTrait,
    QueryFilter, Set,
};
use shared_common::enums::{
    ApplicationComponentStatus, ApplicationComponentType, ApplicationProcessTrigger,
    AutomationLevel, BusinessValueRating, CapabilityLevel, CapabilityStatus, CostRating,
    LifecycleStatus, MaturityLevel, ValueStreamImportance, CapabilityRealizationTargetType,
};
use shared_common::value_objects::{StringStringMap, StringVec};
use uuid::Uuid;

use business_architecture::infrastructure::persistence::entities::{
    application_component, application_process, application_process_step, business_capability,
    business_process, capability_process, capability_realization,
    process_step, stage_capability, value_stream, value_stream_stage,
};

const TEST_SPACE_ID: Uuid =
    Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10]);

const SEED_TIME: chrono::DateTime<Utc> = chrono::DateTime::from_timestamp(1_577_836_800, 0)
    .expect("2020-01-01T00:00:00Z is a valid timestamp");

const fn id(last: u8) -> Uuid {
    Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, last])
}

pub async fn seed_dogfood(db: &DatabaseConnection) -> anyhow::Result<()> {
    seed_business_architecture(db).await?;
    seed_application_architecture(db).await?;
    seed_realizations(db).await?;
    Ok(())
}

async fn insert_if_missing<E>(
    db: &DatabaseConnection,
    id: Uuid,
    model: E::Model,
) -> anyhow::Result<()>
where
    E: EntityTrait,
    E::Model: IntoActiveModel<E::ActiveModel>,
    E::ActiveModel: ActiveModelTrait + Send,
    <<E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<Uuid>,
{
    if E::find_by_id(id).one(db).await?.is_some() {
        return Ok(());
    }
    model.into_active_model().insert(db).await?;
    Ok(())
}

/// Build a `StringVec` from a slice of string literals.
fn sv(slice: &[&str]) -> StringVec {
    StringVec(slice.iter().map(|s| s.to_string()).collect())
}

mod vs {
    use super::id;
    pub const VALUE_STREAM: super::Uuid = id(0xa0);
    pub const STAGE_DESIGN: super::Uuid = id(0xa1);
    pub const STAGE_DELIVER: super::Uuid = id(0xa2);
    pub const STAGE_OPERATE: super::Uuid = id(0xa3);
}

mod cap {
    use super::id;
    pub const REQ_ANALYSIS: super::Uuid = id(0xb1);
    pub const ARCH_DECISION: super::Uuid = id(0xb2);
    pub const TASK_DISPATCH: super::Uuid = id(0xb3);
    pub const AUTO_FIX: super::Uuid = id(0xb4);
    pub const CI: super::Uuid = id(0xb5);
    pub const CODE_REVIEW: super::Uuid = id(0xb6);
    pub const IMAGE_BUILD: super::Uuid = id(0xb7);
    pub const AUTO_DEPLOY: super::Uuid = id(0xb8);
    pub const SMOKE_TEST: super::Uuid = id(0xb9);
    pub const INCIDENT_RECOVERY: super::Uuid = id(0xba);
}

mod bp {
    use super::id;
    pub const REQUIREMENT_INTAKE: super::Uuid = id(0xc1);
    pub const DESIGN_REVIEW: super::Uuid = id(0xc2);
    pub const TASK_ASSIGNMENT: super::Uuid = id(0xc3);
    pub const CODE_VERIFICATION: super::Uuid = id(0xc4);
    pub const AUTO_REMEDIATION: super::Uuid = id(0xc5);
    pub const BUILD_RELEASE: super::Uuid = id(0xc6);
    pub const DEPLOYMENT: super::Uuid = id(0xc7);
    pub const SMOKE_VERIFICATION: super::Uuid = id(0xc8);
    pub const INCIDENT_RESPONSE: super::Uuid = id(0xc9);
}

mod ps {
    use super::id;
    pub const INTAKE_PARSE: super::Uuid = id(0xd1);
    pub const INTAKE_LABEL: super::Uuid = id(0xd2);
    pub const DESIGN_DRAFT: super::Uuid = id(0xd3);
    pub const DESIGN_APPROVE: super::Uuid = id(0xd4);
    pub const DISPATCH: super::Uuid = id(0xd5);
    pub const VERIFY_LINT: super::Uuid = id(0xd6);
    pub const VERIFY_TEST: super::Uuid = id(0xd7);
    pub const REMEDIATE: super::Uuid = id(0xd8);
    pub const BUILD: super::Uuid = id(0xd9);
    pub const PUSH_IMAGE: super::Uuid = id(0xda);
    pub const DEPLOY: super::Uuid = id(0xdb);
    pub const SMOKE_RUN: super::Uuid = id(0xdc);
    pub const DETECT: super::Uuid = id(0xdd);
    pub const RECOVER: super::Uuid = id(0xde);
}

mod ac {
    use super::id;
    pub const PR_CI: super::Uuid = id(0xe1);
    pub const E2E_TEST: super::Uuid = id(0xe2);
    pub const FIX: super::Uuid = id(0xe3);
    pub const FIX_ISSUE: super::Uuid = id(0xe4);
    pub const ON_STOP: super::Uuid = id(0xe5);
    pub const REVIEW_AI: super::Uuid = id(0xe6);
    pub const ON_PUSH_BUILD: super::Uuid = id(0xe7);
    pub const SYNC_DEPLOY: super::Uuid = id(0xe8);
    pub const ON_LABEL: super::Uuid = id(0xe9);
}

mod ap {
    use super::id;
    pub const PR_CI_PIPELINE: super::Uuid = id(0xf1);
    pub const AUTO_FIX_PIPELINE: super::Uuid = id(0xf2);
    pub const BUILD_PIPELINE: super::Uuid = id(0xf3);
    pub const DEPLOY_PIPELINE: super::Uuid = id(0xf4);
}

mod aps {
    use super::id;
    pub const PR_LINT: super::Uuid = id(0x21);
    pub const PR_UNIT: super::Uuid = id(0x22);
    pub const PR_REVIEW: super::Uuid = id(0x23);
    pub const PR_E2E: super::Uuid = id(0x24);
    pub const FIX_TRIGGER: super::Uuid = id(0x25);
    pub const FIX_RUN: super::Uuid = id(0x26);
    pub const BUILD_COMPILE: super::Uuid = id(0x27);
    pub const BUILD_PUSH: super::Uuid = id(0x28);
    pub const DEPLOY_SYNC: super::Uuid = id(0x29);
    pub const DEPLOY_STOP: super::Uuid = id(0x2a);
}

async fn seed_business_architecture(db: &DatabaseConnection) -> anyhow::Result<()> {
    let now = SEED_TIME;

    insert_if_missing::<value_stream::Entity>(
        db,
        vs::VALUE_STREAM,
        value_stream::Model {
            id: vs::VALUE_STREAM,
            logical_id: vs::VALUE_STREAM,
            business_version: "1.0.0".into(),
            status: LifecycleStatus::Active,
            name: "产品交付".into(),
            description: Some("EAP 平台自身的端到端产品交付价值流".into()),
            triggering_event: Some("用户需求".into()),
            end_deliverable: Some("可用的产品版本".into()),
            owner_id: None,
            importance: ValueStreamImportance::Critical,
            stakeholders: StringVec(vec!["产品负责人".into(), "架构师".into()]),
            performance_metrics: StringStringMap(
                [("lead_time".into(), "周期时间".into())].into_iter().collect(),
            ),
            created_by: None,
            updated_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            space_id: TEST_SPACE_ID,
        },
    )
    .await?;

    for (sid, name, seq, input, output) in [
        (vs::STAGE_DESIGN, "需求设计", 1, "用户需求", "Issue+设计"),
        (vs::STAGE_DELIVER, "开发交付", 2, "Issue", "部署镜像"),
        (vs::STAGE_OPERATE, "部署运营", 3, "部署镜像", "运行中的产品"),
    ] {
        insert_if_missing::<value_stream_stage::Entity>(
            db,
            sid,
            value_stream_stage::Model {
                id: sid,
                name: name.into(),
                sequence_order: seq,
                input: Some(input.into()),
                output: Some(output.into()),
                description: None,
                objective_metrics: Default::default(),
                entry_criteria: None,
                exit_criteria: None,
                owner_id: None,
                key_metrics: Default::default(),
                value_stream_id: vs::VALUE_STREAM,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
        )
        .await?;
    }

    for (cid, name, desc, level, maturity, value) in capabilities() {
        insert_if_missing::<business_capability::Entity>(
            db,
            cid,
            business_capability::Model {
                id: cid,
                logical_id: cid,
                business_version: "1.0.0".into(),
                status: LifecycleStatus::Active,
                capability_status: CapabilityStatus::Active,
                name: name.into(),
                description: desc.into(),
                level,
                maturity,
                business_value: value,
                cost: CostRating::Medium,
                metrics: None,
                owner_id: None,
                created_by: None,
                updated_by: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                space_id: TEST_SPACE_ID,
            },
        )
        .await?;
    }

    for spec in business_processes() {
        insert_if_missing::<business_process::Entity>(
            db,
            spec.id,
            business_process::Model {
                id: spec.id,
                logical_id: spec.id,
                business_version: "1.0.0".into(),
                status: LifecycleStatus::Active,
                name: spec.name.into(),
                description: spec.description.into(),
                sla: spec.sla.map(Into::into),
                cost_per_transaction: None,
                cycle_time: None,
                automation_level: spec.automation,
                maturity: spec.maturity,
                owner_id: None,
                created_by: None,
                updated_by: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                space_id: TEST_SPACE_ID,
            },
        )
        .await?;

        for step in &spec.steps {
            insert_if_missing::<process_step::Entity>(
                db,
                step.id,
                process_step::Model {
                    id: step.id,
                    name: step.name.into(),
                    description: step.description.into(),
                    sequence_order: step.seq,
                    business_rules: sv(step.rules),
                    required_inputs: sv(step.inputs),
                    produced_outputs: sv(step.outputs),
                    role_id: None,
                    process_id: spec.id,
                    created_at: now,
                    updated_at: now,
                    deleted_at: None,
                },
            )
            .await?;
        }
    }

    for (capability_id, process_id) in capability_process_links() {
        link_capability_process(db, capability_id, process_id).await?;
    }

    for (stage_id, capability_id) in stage_capability_links() {
        link_stage_capability(db, stage_id, capability_id).await?;
    }

    Ok(())
}

fn capabilities() -> [(Uuid, &'static str, &'static str, CapabilityLevel, MaturityLevel, BusinessValueRating); 10] {
    [
        (cap::REQ_ANALYSIS, "需求分析", "解析用户需求并拆分为可执行的工作项", CapabilityLevel::L1, MaturityLevel::Level2, BusinessValueRating::High),
        (cap::ARCH_DECISION, "架构决策", "记录并维护架构决策记录 (ADR)", CapabilityLevel::L1, MaturityLevel::Level1, BusinessValueRating::High),
        (cap::TASK_DISPATCH, "任务分发", "将 Issue 自动分配给合适的开发人员", CapabilityLevel::L2, MaturityLevel::Level3, BusinessValueRating::Medium),
        (cap::AUTO_FIX, "自动修复", "AI 智能体根据失败信号自动提交修复", CapabilityLevel::L2, MaturityLevel::Level3, BusinessValueRating::High),
        (cap::CI, "持续集成", "PR 合并前运行 lint、单元测试与构建", CapabilityLevel::L2, MaturityLevel::Level3, BusinessValueRating::High),
        (cap::CODE_REVIEW, "代码审查", "AI 辅助代码审查与人工复核", CapabilityLevel::L2, MaturityLevel::Level3, BusinessValueRating::Medium),
        (cap::IMAGE_BUILD, "镜像构建", "构建并推送容器镜像至镜像仓库", CapabilityLevel::L2, MaturityLevel::Level4, BusinessValueRating::High),
        (cap::AUTO_DEPLOY, "自动部署", "将镜像部署至目标环境", CapabilityLevel::L2, MaturityLevel::Level3, BusinessValueRating::High),
        (cap::SMOKE_TEST, "冒烟验证", "部署后执行冒烟测试套件", CapabilityLevel::L2, MaturityLevel::Level4, BusinessValueRating::High),
        (cap::INCIDENT_RECOVERY, "故障恢复", "检测故障并自动回滚或重启", CapabilityLevel::L2, MaturityLevel::Level2, BusinessValueRating::Medium),
    ]
}

struct ProcessSpec {
    id: Uuid,
    name: &'static str,
    description: &'static str,
    sla: Option<&'static str>,
    automation: Option<AutomationLevel>,
    maturity: Option<MaturityLevel>,
    steps: Vec<ProcessStepSpec>,
}

struct ProcessStepSpec {
    id: Uuid,
    name: &'static str,
    description: &'static str,
    seq: i32,
    rules: &'static [&'static str],
    inputs: &'static [&'static str],
    outputs: &'static [&'static str],
}

const fn step(
    id: Uuid,
    name: &'static str,
    description: &'static str,
    seq: i32,
    rules: &'static [&'static str],
    inputs: &'static [&'static str],
    outputs: &'static [&'static str],
) -> ProcessStepSpec {
    ProcessStepSpec { id, name, description, seq, rules, inputs, outputs }
}

fn business_processes() -> [ProcessSpec; 9] {
    [
        ProcessSpec {
            id: bp::REQUIREMENT_INTAKE,
            name: "需求受理",
            description: "接收并结构化用户需求",
            sla: Some("1 工作日"),
            automation: Some(AutomationLevel::SemiAutomated),
            maturity: Some(MaturityLevel::Level2),
            steps: vec![
                step(ps::INTAKE_PARSE, "解析需求", "将原始需求解析为结构化 Issue", 1, &["必须包含验收标准"], &["用户需求"], &["Issue"]),
                step(ps::INTAKE_LABEL, "打标分发", "为 Issue 打上类型与优先级标签", 2, &["按团队领域打标"], &["Issue"], &["已标注 Issue"]),
            ],
        },
        ProcessSpec {
            id: bp::DESIGN_REVIEW,
            name: "设计评审",
            description: "评审架构设计方案并产出 ADR",
            sla: Some("2 工作日"),
            automation: Some(AutomationLevel::Manual),
            maturity: Some(MaturityLevel::Level1),
            steps: vec![
                step(ps::DESIGN_DRAFT, "起草设计", "架构师起草方案文档", 1, &["需覆盖非功能性需求"], &["已标注 Issue"], &["设计草案"]),
                step(ps::DESIGN_APPROVE, "评审定稿", "评审委员会确认方案", 2, &["至少两人评审"], &["设计草案"], &["ADR"]),
            ],
        },
        ProcessSpec {
            id: bp::TASK_ASSIGNMENT,
            name: "任务分配",
            description: "将 Issue 分配给开发人员",
            sla: Some("0.5 工作日"),
            automation: Some(AutomationLevel::FullyAutomated),
            maturity: Some(MaturityLevel::Level3),
            steps: vec![
                step(ps::DISPATCH, "自动派单", "按领域与负载自动分配", 1, &["负载均衡"], &["已标注 Issue"], &["已分配 Issue"]),
            ],
        },
        ProcessSpec {
            id: bp::CODE_VERIFICATION,
            name: "代码验证",
            description: "PR 合并前的 lint、测试与审查",
            sla: Some("0.5 工作日"),
            automation: Some(AutomationLevel::FullyAutomated),
            maturity: Some(MaturityLevel::Level3),
            steps: vec![
                step(ps::VERIFY_LINT, "静态检查", "运行 clippy 与 fmt 检查", 1, &["零警告"], &["PR"], &["lint 报告"]),
                step(ps::VERIFY_TEST, "单元测试", "运行单元测试套件", 2, &["覆盖率达标"], &["PR"], &["测试报告"]),
            ],
        },
        ProcessSpec {
            id: bp::AUTO_REMEDIATION,
            name: "自动修复",
            description: "AI 智能体自动修复失败用例",
            sla: Some("1 工作日"),
            automation: Some(AutomationLevel::FullyAutomated),
            maturity: Some(MaturityLevel::Level3),
            steps: vec![
                step(ps::REMEDIATE, "生成修复", "根据失败信号生成并提交修复 PR", 1, &["必须通过 CI"], &["失败信号"], &["修复 PR"]),
            ],
        },
        ProcessSpec {
            id: bp::BUILD_RELEASE,
            name: "构建发布",
            description: "构建镜像并推送至仓库",
            sla: Some("0.5 工作日"),
            automation: Some(AutomationLevel::FullyAutomated),
            maturity: Some(MaturityLevel::Level4),
            steps: vec![
                step(ps::BUILD, "编译构建", "编译产物并打包镜像", 1, &["可复现构建"], &["PR"], &["镜像"]),
                step(ps::PUSH_IMAGE, "推送镜像", "推送镜像至镜像仓库", 2, &["签名校验"], &["镜像"], &["已发布镜像"]),
            ],
        },
        ProcessSpec {
            id: bp::DEPLOYMENT,
            name: "部署",
            description: "将镜像部署至目标环境",
            sla: Some("0.5 工作日"),
            automation: Some(AutomationLevel::FullyAutomated),
            maturity: Some(MaturityLevel::Level3),
            steps: vec![
                step(ps::DEPLOY, "执行部署", "将镜像部署至运行环境", 1, &["蓝绿或滚动"], &["已发布镜像"], &["运行实例"]),
            ],
        },
        ProcessSpec {
            id: bp::SMOKE_VERIFICATION,
            name: "冒烟验证",
            description: "部署后执行冒烟测试",
            sla: Some("0.25 工作日"),
            automation: Some(AutomationLevel::FullyAutomated),
            maturity: Some(MaturityLevel::Level4),
            steps: vec![
                step(ps::SMOKE_RUN, "运行冒烟", "执行冒烟测试套件", 1, &["全部通过"], &["运行实例"], &["冒烟报告"]),
            ],
        },
        ProcessSpec {
            id: bp::INCIDENT_RESPONSE,
            name: "故障响应",
            description: "检测故障并自动恢复",
            sla: Some("0.25 工作日"),
            automation: Some(AutomationLevel::SemiAutomated),
            maturity: Some(MaturityLevel::Level2),
            steps: vec![
                step(ps::DETECT, "故障检测", "监控告警触发检测", 1, &["5 分钟内告警"], &["监控指标"], &["告警"]),
                step(ps::RECOVER, "自动恢复", "回滚或重启恢复服务", 2, &["优先回滚"], &["告警"], &["恢复确认"]),
            ],
        },
    ]
}

fn capability_process_links() -> [(Uuid, Uuid); 11] {
    [
        (cap::REQ_ANALYSIS, bp::REQUIREMENT_INTAKE),
        (cap::ARCH_DECISION, bp::REQUIREMENT_INTAKE),
        (cap::ARCH_DECISION, bp::DESIGN_REVIEW),
        (cap::TASK_DISPATCH, bp::TASK_ASSIGNMENT),
        (cap::AUTO_FIX, bp::AUTO_REMEDIATION),
        (cap::CI, bp::CODE_VERIFICATION),
        (cap::CODE_REVIEW, bp::CODE_VERIFICATION),
        (cap::IMAGE_BUILD, bp::BUILD_RELEASE),
        (cap::AUTO_DEPLOY, bp::DEPLOYMENT),
        (cap::SMOKE_TEST, bp::SMOKE_VERIFICATION),
        (cap::INCIDENT_RECOVERY, bp::INCIDENT_RESPONSE),
    ]
}

fn stage_capability_links() -> [(Uuid, Uuid); 10] {
    [
        (vs::STAGE_DESIGN, cap::REQ_ANALYSIS),
        (vs::STAGE_DESIGN, cap::ARCH_DECISION),
        (vs::STAGE_DESIGN, cap::TASK_DISPATCH),
        (vs::STAGE_DELIVER, cap::AUTO_FIX),
        (vs::STAGE_DELIVER, cap::CI),
        (vs::STAGE_DELIVER, cap::CODE_REVIEW),
        (vs::STAGE_DELIVER, cap::IMAGE_BUILD),
        (vs::STAGE_OPERATE, cap::AUTO_DEPLOY),
        (vs::STAGE_OPERATE, cap::SMOKE_TEST),
        (vs::STAGE_OPERATE, cap::INCIDENT_RECOVERY),
    ]
}

async fn link_capability_process(
    db: &DatabaseConnection,
    capability_id: Uuid,
    process_id: Uuid,
) -> anyhow::Result<()> {
    let exists = capability_process::Entity::find()
        .filter(capability_process::Column::CapabilityId.eq(capability_id))
        .filter(capability_process::Column::ProcessId.eq(process_id))
        .one(db)
        .await?
        .is_some();
    if !exists {
        capability_process::ActiveModel {
            capability_id: Set(capability_id),
            process_id: Set(process_id),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn link_stage_capability(
    db: &DatabaseConnection,
    stage_id: Uuid,
    capability_id: Uuid,
) -> anyhow::Result<()> {
    let exists = stage_capability::Entity::find()
        .filter(stage_capability::Column::StageId.eq(stage_id))
        .filter(stage_capability::Column::CapabilityId.eq(capability_id))
        .one(db)
        .await?
        .is_some();
    if !exists {
        stage_capability::ActiveModel {
            stage_id: Set(stage_id),
            capability_id: Set(capability_id),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// P5: application architecture
// ---------------------------------------------------------------------------

async fn seed_application_architecture(db: &DatabaseConnection) -> anyhow::Result<()> {
    let now = SEED_TIME;

    for (cid, name, typ, repo, path) in application_components() {
        insert_if_missing::<application_component::Entity>(
            db,
            cid,
            application_component::Model {
                id: cid,
                name: name.into(),
                r#type: typ,
                repo: repo.into(),
                path: path.into(),
                technology: Some("GitHub Actions / Python".into()),
                status: ApplicationComponentStatus::Active,
                version: "1.0.0".into(),
                owner_id: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
                space_id: TEST_SPACE_ID,
            },
        )
        .await?;
    }

    for spec in application_processes() {
        insert_if_missing::<application_process::Entity>(
            db,
            spec.id,
            application_process::Model {
                id: spec.id,
                name: spec.name.into(),
                description: spec.description.into(),
                trigger: spec.trigger,
                inputs: sv(spec.inputs),
                outputs: sv(spec.outputs),
                timeout: spec.timeout,
                retry: spec.retry,
                status: LifecycleStatus::Active,
                logical_id: spec.id,
                business_version: "1.0.0".into(),
                created_at: now,
                updated_at: now,
                deleted_at: None,
                space_id: TEST_SPACE_ID,
            },
        )
        .await?;

        for st in &spec.steps {
            insert_if_missing::<application_process_step::Entity>(
                db,
                st.id,
                application_process_step::Model {
                    id: st.id,
                    name: st.name.into(),
                    action: st.action.into(),
                    description: st.description.into(),
                    sequence_order: st.seq,
                    inputs: sv(st.inputs),
                    outputs: sv(st.outputs),
                    dependencies: sv(st.deps),
                    process_id: spec.id,
                    created_at: now,
                    updated_at: now,
                    deleted_at: None,
                },
            )
            .await?;
        }
    }

    Ok(())
}

fn application_components() -> [(Uuid, &'static str, ApplicationComponentType, &'static str, &'static str); 9] {
    [
        (ac::PR_CI, "pr-ci.yml", ApplicationComponentType::Workflow, "link-seek/enterprise-architecture-platform", ".github/workflows/pr-ci.yml"),
        (ac::E2E_TEST, "e2e-test.yml", ApplicationComponentType::Workflow, "link-seek/enterprise-architecture-platform", ".github/workflows/e2e-test.yml"),
        (ac::FIX, "fix.yml", ApplicationComponentType::Workflow, "link-seek/enterprise-architecture-platform", ".github/workflows/fix.yml"),
        (ac::FIX_ISSUE, "fix_issue.py", ApplicationComponentType::Script, "link-seek/enterprise-architecture-platform", "scripts/fix_issue.py"),
        (ac::ON_STOP, "on_stop.sh", ApplicationComponentType::Script, "link-seek/enterprise-architecture-platform", "scripts/on_stop.sh"),
        (ac::REVIEW_AI, "review-ai", ApplicationComponentType::Service, "link-seek/enterprise-architecture-platform", "services/review-ai"),
        (ac::ON_PUSH_BUILD, "on-push-build.yml", ApplicationComponentType::Workflow, "link-seek/enterprise-architecture-platform", ".github/workflows/on-push-build.yml"),
        (ac::SYNC_DEPLOY, "sync-deploy.yml", ApplicationComponentType::Workflow, "link-seek/enterprise-architecture-platform", ".github/workflows/sync-deploy.yml"),
        (ac::ON_LABEL, "on-label.yml", ApplicationComponentType::Workflow, "link-seek/enterprise-architecture-platform", ".github/workflows/on-label.yml"),
    ]
}

struct AppProcessSpec {
    id: Uuid,
    name: &'static str,
    description: &'static str,
    trigger: ApplicationProcessTrigger,
    inputs: &'static [&'static str],
    outputs: &'static [&'static str],
    timeout: Option<i32>,
    retry: Option<i32>,
    steps: Vec<AppProcessStepSpec>,
}

struct AppProcessStepSpec {
    id: Uuid,
    name: &'static str,
    action: &'static str,
    description: &'static str,
    seq: i32,
    inputs: &'static [&'static str],
    outputs: &'static [&'static str],
    deps: &'static [&'static str],
}

const fn astep(
    id: Uuid,
    name: &'static str,
    action: &'static str,
    description: &'static str,
    seq: i32,
    inputs: &'static [&'static str],
    outputs: &'static [&'static str],
    deps: &'static [&'static str],
) -> AppProcessStepSpec {
    AppProcessStepSpec { id, name, action, description, seq, inputs, outputs, deps }
}

fn application_processes() -> [AppProcessSpec; 4] {
    [
        AppProcessSpec {
            id: ap::PR_CI_PIPELINE,
            name: "PR CI Pipeline",
            description: "PR 触发的 lint、测试、审查与 e2e 流水线",
            trigger: ApplicationProcessTrigger::PullRequest,
            inputs: &["PR"],
            outputs: &["CI 报告"],
            timeout: Some(1800),
            retry: Some(1),
            steps: vec![
                astep(aps::PR_LINT, "Lint", "run lint", "运行 clippy 与 fmt", 1, &["PR"], &["lint 报告"], &[]),
                astep(aps::PR_UNIT, "Unit Test", "run unit tests", "运行单元测试", 2, &["PR"], &["测试报告"], &["lint 报告"]),
                astep(aps::PR_REVIEW, "AI Review", "run review-ai", "AI 代码审查", 3, &["PR"], &["审查意见"], &["测试报告"]),
                astep(aps::PR_E2E, "E2E Test", "run e2e-test.yml", "端到端测试", 4, &["PR"], &["e2e 报告"], &["审查意见"]),
            ],
        },
        AppProcessSpec {
            id: ap::AUTO_FIX_PIPELINE,
            name: "Auto-fix Pipeline",
            description: "失败信号触发的自动修复流水线",
            trigger: ApplicationProcessTrigger::Webhook,
            inputs: &["失败信号"],
            outputs: &["修复 PR"],
            timeout: Some(3600),
            retry: Some(2),
            steps: vec![
                astep(aps::FIX_TRIGGER, "Trigger", "receive webhook", "接收失败信号", 1, &["失败信号"], &["修复任务"], &[]),
                astep(aps::FIX_RUN, "Run fix_issue.py", "run fix_issue.py", "生成并提交修复", 2, &["修复任务"], &["修复 PR"], &["修复任务"]),
            ],
        },
        AppProcessSpec {
            id: ap::BUILD_PIPELINE,
            name: "Build Pipeline",
            description: "推送触发的构建与镜像发布流水线",
            trigger: ApplicationProcessTrigger::Push,
            inputs: &["代码推送"],
            outputs: &["已发布镜像"],
            timeout: Some(2400),
            retry: Some(1),
            steps: vec![
                astep(aps::BUILD_COMPILE, "Compile", "run build", "编译并打包镜像", 1, &["代码推送"], &["镜像"], &[]),
                astep(aps::BUILD_PUSH, "Push Image", "push image", "推送镜像至仓库", 2, &["镜像"], &["已发布镜像"], &["镜像"]),
            ],
        },
        AppProcessSpec {
            id: ap::DEPLOY_PIPELINE,
            name: "Deploy Pipeline",
            description: "定时同步部署与停止流水线",
            trigger: ApplicationProcessTrigger::Schedule,
            inputs: &["已发布镜像"],
            outputs: &["运行实例"],
            timeout: Some(1200),
            retry: Some(1),
            steps: vec![
                astep(aps::DEPLOY_SYNC, "Sync Deploy", "run sync-deploy.yml", "同步部署镜像", 1, &["已发布镜像"], &["运行实例"], &[]),
                astep(aps::DEPLOY_STOP, "On Stop", "run on_stop.sh", "停止时清理资源", 2, &["停止信号"], &["清理确认"], &[]),
            ],
        },
    ]
}

// ---------------------------------------------------------------------------
// P5: realizes mappings (cross-layer)
// ---------------------------------------------------------------------------

async fn seed_realizations(db: &DatabaseConnection) -> anyhow::Result<()> {
    // v2.1: ProcessRealization and StepRealization are deleted.
    // CapabilityRealization now targets a process (business or application)
    // instead of an application component. We link capabilities to the
    // business processes that enable them.
    for (cap_id, bp_id) in [
        (cap::CI, bp::CODE_VERIFICATION),
        (cap::CODE_REVIEW, bp::CODE_VERIFICATION),
        (cap::AUTO_FIX, bp::AUTO_REMEDIATION),
        (cap::IMAGE_BUILD, bp::BUILD_RELEASE),
        (cap::AUTO_DEPLOY, bp::DEPLOYMENT),
        (cap::SMOKE_TEST, bp::SMOKE_VERIFICATION),
        (cap::INCIDENT_RECOVERY, bp::SMOKE_VERIFICATION),
        (cap::TASK_DISPATCH, bp::TASK_ASSIGNMENT),
    ] {
        link_capability_realization(db, cap_id, bp_id, CapabilityRealizationTargetType::BusinessProcess).await?;
    }

    Ok(())
}

async fn link_capability_realization(
    db: &DatabaseConnection,
    capability_id: Uuid,
    process_id: Uuid,
    process_type: CapabilityRealizationTargetType,
) -> anyhow::Result<()> {
    let exists = capability_realization::Entity::find()
        .filter(capability_realization::Column::CapabilityId.eq(capability_id))
        .filter(capability_realization::Column::ProcessId.eq(process_id))
        .filter(capability_realization::Column::ProcessType.eq(process_type))
        .one(db)
        .await?
        .is_some();
    if !exists {
        capability_realization::ActiveModel {
            capability_id: Set(capability_id),
            process_id: Set(process_id),
            process_type: Set(process_type),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::MigratorTrait;
    use sea_orm::{EntityTrait, PaginatorTrait};

    async fn setup_db() -> DatabaseConnection {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        db
    }

    #[tokio::test]
    async fn seed_creates_expected_business_architecture() {
        let db = setup_db().await;
        seed_dogfood(&db).await.unwrap();

        assert_eq!(
            value_stream::Entity::find().count(&db).await.unwrap(),
            1,
            "value streams"
        );
        assert_eq!(
            value_stream_stage::Entity::find().count(&db).await.unwrap(),
            3,
            "stages"
        );
        assert_eq!(
            business_capability::Entity::find().count(&db).await.unwrap(),
            10,
            "capabilities"
        );
        assert_eq!(
            business_process::Entity::find().count(&db).await.unwrap(),
            9,
            "business processes"
        );
        assert!(
            process_step::Entity::find().count(&db).await.unwrap() > 0,
            "process steps"
        );
        assert_eq!(
            capability_process::Entity::find().count(&db).await.unwrap(),
            11,
            "every capability linked to at least one process"
        );
        assert_eq!(
            stage_capability::Entity::find().count(&db).await.unwrap(),
            10,
            "stage-capability links"
        );
    }

    #[tokio::test]
    async fn seed_creates_expected_application_architecture() {
        let db = setup_db().await;
        seed_dogfood(&db).await.unwrap();

        assert_eq!(
            application_component::Entity::find().count(&db).await.unwrap(),
            9,
            "application components"
        );
        assert_eq!(
            application_process::Entity::find().count(&db).await.unwrap(),
            4,
            "application processes"
        );
        assert!(
            application_process_step::Entity::find().count(&db).await.unwrap() > 0,
            "application process steps"
        );
    }

    #[tokio::test]
    async fn seed_creates_realization_mappings() {
        let db = setup_db().await;
        seed_dogfood(&db).await.unwrap();

        assert_eq!(
            capability_realization::Entity::find().count(&db).await.unwrap(),
            8,
            "capability-level realizes mappings (v2.1: Capability -> Process)"
        );
    }

    #[tokio::test]
    async fn seed_is_idempotent() {
        let db = setup_db().await;
        seed_dogfood(&db).await.unwrap();
        let vs1 = value_stream::Entity::find().count(&db).await.unwrap();
        let cap1 = business_capability::Entity::find().count(&db).await.unwrap();
        let comp1 = application_component::Entity::find().count(&db).await.unwrap();
        let proc1 = business_process::Entity::find().count(&db).await.unwrap();
        let step1 = process_step::Entity::find().count(&db).await.unwrap();
        let areal1 = capability_realization::Entity::find().count(&db).await.unwrap();

        // Re-run; counts must not change.
        seed_dogfood(&db).await.unwrap();
        assert_eq!(
            value_stream::Entity::find().count(&db).await.unwrap(),
            vs1
        );
        assert_eq!(
            business_capability::Entity::find().count(&db).await.unwrap(),
            cap1
        );
        assert_eq!(
            application_component::Entity::find().count(&db).await.unwrap(),
            comp1
        );
        assert_eq!(
            business_process::Entity::find().count(&db).await.unwrap(),
            proc1
        );
        assert_eq!(
            process_step::Entity::find().count(&db).await.unwrap(),
            step1
        );
        assert_eq!(
            capability_realization::Entity::find().count(&db).await.unwrap(),
            areal1,
            "realization mappings must not duplicate"
        );
    }
}
