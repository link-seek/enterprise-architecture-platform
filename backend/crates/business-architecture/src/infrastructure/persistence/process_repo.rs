use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, TransactionTrait,
};
use shared_common::enums::LifecycleStatus;
use uuid::Uuid;

use crate::application::version::bump_minor;
use crate::domain::error::DomainError;
use crate::domain::process::entity::{
    AffectedProcessLink, BusinessProcess, ProcessStep, PublishVersionResult,
};
use crate::domain::process::repository::{ProcessRepository, ProcessStepRepository};
use crate::infrastructure::persistence::entities::{business_capability, business_process, capability_process, process_step};

impl From<business_process::Model> for BusinessProcess {
    fn from(m: business_process::Model) -> Self {
        BusinessProcess {
            id: m.id,
            logical_id: m.logical_id,
            business_version: m.business_version,
            status: m.status,
            name: m.name,
            description: m.description,
            inputs: m.inputs,
            outputs: m.outputs,
            sla: m.sla,
            cost_per_transaction: m.cost_per_transaction,
            cycle_time: m.cycle_time,
            automation_level: m.automation_level,
            maturity: m.maturity,
            owner_id: m.owner_id,
            created_by: m.created_by,
            updated_by: m.updated_by,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
            space_id: m.space_id,
        }
    }
}

impl From<&BusinessProcess> for business_process::Model {
    fn from(p: &BusinessProcess) -> Self {
        business_process::Model {
            id: p.id,
            logical_id: p.logical_id,
            business_version: p.business_version.clone(),
            status: p.status,
            name: p.name.clone(),
            description: p.description.clone(),
            inputs: p.inputs.clone(),
            outputs: p.outputs.clone(),
            sla: p.sla.clone(),
            cost_per_transaction: p.cost_per_transaction,
            cycle_time: p.cycle_time,
            automation_level: p.automation_level,
            maturity: p.maturity,
            owner_id: p.owner_id,
            created_by: p.created_by,
            updated_by: p.updated_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
            deleted_at: p.deleted_at,
            space_id: p.space_id,
        }
    }
}

impl From<process_step::Model> for ProcessStep {
    fn from(m: process_step::Model) -> Self {
        ProcessStep {
            id: m.id,
            name: m.name,
            description: m.description,
            sequence_order: m.sequence_order,
            business_rules: m.business_rules,
            required_inputs: m.required_inputs,
            produced_outputs: m.produced_outputs,
            role_id: m.role_id,
            process_id: m.process_id,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }
    }
}

pub struct SeaOrmProcessRepo {
    db: DatabaseConnection,
}

impl SeaOrmProcessRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ProcessRepository for SeaOrmProcessRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<BusinessProcess>, DomainError> {
        let model = business_process::Entity::find()
            .filter(business_process::Column::Id.eq(id))
            .filter(business_process::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn find_active_by_logical_id(
        &self,
        logical_id: Uuid,
    ) -> Result<Option<BusinessProcess>, DomainError> {
        let model = business_process::Entity::find()
            .filter(business_process::Column::LogicalId.eq(logical_id))
            .filter(business_process::Column::Status.eq(LifecycleStatus::Active))
            .filter(business_process::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn find_all_versions(
        &self,
        logical_id: Uuid,
    ) -> Result<Vec<BusinessProcess>, DomainError> {
        let models = business_process::Entity::find()
            .filter(business_process::Column::LogicalId.eq(logical_id))
            .filter(business_process::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn find_all_active(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<BusinessProcess>, u64), DomainError> {
        let paginator = business_process::Entity::find()
            .filter(business_process::Column::DeletedAt.is_null())
            .filter(business_process::Column::Status.eq(LifecycleStatus::Active))
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let models = paginator.fetch_page(page.saturating_sub(1)).await?;

        let procs = models.into_iter().map(Into::into).collect();
        Ok((procs, total))
    }

    async fn save(&self, proc: &BusinessProcess) -> Result<BusinessProcess, DomainError> {
        let existing = business_process::Entity::find_by_id(proc.id)
            .one(&self.db)
            .await?;

        let result = if let Some(model) = existing {
            let mut active: business_process::ActiveModel = model.into();
            active.business_version = Set(proc.business_version.clone());
            active.status = Set(proc.status);
            active.name = Set(proc.name.clone());
            active.description = Set(proc.description.clone());
            active.inputs = Set(proc.inputs.clone());
            active.outputs = Set(proc.outputs.clone());
            active.sla = Set(proc.sla.clone());
            active.cost_per_transaction = Set(proc.cost_per_transaction);
            active.cycle_time = Set(proc.cycle_time);
            active.automation_level = Set(proc.automation_level);
            active.maturity = Set(proc.maturity);
            active.owner_id = Set(proc.owner_id);
            active.updated_by = Set(proc.updated_by);
            active.updated_at = Set(proc.updated_at);
            active.update(&self.db).await?
        } else {
            let active = business_process::ActiveModel {
                id: Set(proc.id),
                logical_id: Set(proc.logical_id),
                business_version: Set(proc.business_version.clone()),
                status: Set(proc.status),
                name: Set(proc.name.clone()),
                description: Set(proc.description.clone()),
                inputs: Set(proc.inputs.clone()),
                outputs: Set(proc.outputs.clone()),
                sla: Set(proc.sla.clone()),
                cost_per_transaction: Set(proc.cost_per_transaction),
                cycle_time: Set(proc.cycle_time),
                automation_level: Set(proc.automation_level),
                maturity: Set(proc.maturity),
                owner_id: Set(proc.owner_id),
                created_by: Set(proc.created_by),
                updated_by: Set(proc.updated_by),
                created_at: Set(proc.created_at),
                updated_at: Set(proc.updated_at),
                deleted_at: Set(None),
                space_id: Set(proc.space_id),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn archive(&self, id: Uuid) -> Result<(), DomainError> {
        let model = business_process::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ProcessNotFound)?;

        // Enforce the `Deprecated → Archived` edge so the compatibility window
        // cannot be skipped by archiving an `Active` process directly.
        if model.status != LifecycleStatus::Deprecated {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", model.status),
                to: "Archived".to_string(),
                entity: "BusinessProcess".to_string(),
            });
        }

        let mut active: business_process::ActiveModel = model.into();
        active.status = Set(LifecycleStatus::Archived);
        active.updated_at = Set(chrono::Utc::now());
        active.update(&self.db).await?;

        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = business_process::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ProcessNotFound)?;

        let mut active: business_process::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }
}

#[async_trait]
impl ProcessStepRepository for SeaOrmProcessRepo {
    async fn find_by_process(&self, process_id: Uuid) -> Result<Vec<ProcessStep>, DomainError> {
        let models = process_step::Entity::find()
            .filter(process_step::Column::ProcessId.eq(process_id))
            .filter(process_step::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn save(&self, step: &ProcessStep) -> Result<ProcessStep, DomainError> {
        let existing = process_step::Entity::find_by_id(step.id)
            .one(&self.db)
            .await?;

        let result = if let Some(model) = existing {
            let mut active: process_step::ActiveModel = model.into();
            active.name = Set(step.name.clone());
            active.description = Set(step.description.clone());
            active.sequence_order = Set(step.sequence_order);
            active.business_rules = Set(step.business_rules.clone());
            active.required_inputs = Set(step.required_inputs.clone());
            active.produced_outputs = Set(step.produced_outputs.clone());
            active.role_id = Set(step.role_id);
            active.updated_at = Set(step.updated_at);
            active.update(&self.db).await?
        } else {
            let active = process_step::ActiveModel {
                id: Set(step.id),
                name: Set(step.name.clone()),
                description: Set(step.description.clone()),
                sequence_order: Set(step.sequence_order),
                business_rules: Set(step.business_rules.clone()),
                required_inputs: Set(step.required_inputs.clone()),
                produced_outputs: Set(step.produced_outputs.clone()),
                role_id: Set(step.role_id),
                process_id: Set(step.process_id),
                created_at: Set(step.created_at),
                updated_at: Set(step.updated_at),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = process_step::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ProcessNotFound)?;

        let mut active: process_step::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }
}

impl SeaOrmProcessRepo {
    /// Publish a new minor version of the active process identified by
    /// `logical_id`. The old active version transitions to `Deprecated`
    /// (compatibility window) instead of being archived immediately; a new
    /// `Active` row is inserted with the same `logical_id` and `bump_minor`
    /// version. All of this runs inside a single transaction.
    ///
    /// The result also carries the capability links that still reference the
    /// old (now deprecated) version, so the caller can warn about version
    /// anchoring before/after the publish.
    pub async fn publish_new_version(
        &self,
        logical_id: Uuid,
    ) -> Result<PublishVersionResult, DomainError> {
        let txn = self.db.begin().await?;

        let old = business_process::Entity::find()
            .filter(business_process::Column::LogicalId.eq(logical_id))
            .filter(business_process::Column::Status.eq(LifecycleStatus::Active))
            .filter(business_process::Column::DeletedAt.is_null())
            .one(&txn)
            .await?
            .ok_or(DomainError::ProcessVersionNotFound)?;

        let new_version = bump_minor(&old.business_version)?;

        // Collect capability links pointing at the old version row before it is
        // deprecated (transactions see a consistent snapshot).
        let links = capability_process::Entity::find()
            .filter(capability_process::Column::ProcessId.eq(old.id))
            .all(&txn)
            .await?;
        let cap_ids: Vec<Uuid> = links.iter().map(|l| l.capability_id).collect();
        let cap_names: Vec<(Uuid, String)> = if cap_ids.is_empty() {
            Vec::new()
        } else {
            business_capability::Entity::find()
                .filter(business_capability::Column::Id.is_in(cap_ids.clone()))
                .all(&txn)
                .await?
                .into_iter()
                .map(|c| (c.id, c.name))
                .collect()
        };
        let cap_name_by_id: std::collections::HashMap<Uuid, String> =
            cap_names.into_iter().collect();
        let affected_links: Vec<AffectedProcessLink> = links
            .iter()
            .map(|l| AffectedProcessLink {
                capability_id: l.capability_id,
                capability_name: cap_name_by_id
                    .get(&l.capability_id)
                    .cloned()
                    .unwrap_or_default(),
                old_version: old.business_version.clone(),
                new_version: new_version.clone(),
            })
            .collect();

        // Old active version → Deprecated (compatibility window).
        let mut old_active: business_process::ActiveModel = old.clone().into();
        old_active.status = Set(LifecycleStatus::Deprecated);
        old_active.updated_at = Set(chrono::Utc::now());
        old_active.update(&txn).await?;

        let now = chrono::Utc::now();
        let new_id = Uuid::new_v4();
        let new_active = business_process::ActiveModel {
            id: Set(new_id),
            logical_id: Set(logical_id),
            business_version: Set(new_version),
            status: Set(LifecycleStatus::Active),
            name: Set(old.name.clone()),
            description: Set(old.description.clone()),
            inputs: Set(old.inputs.clone()),
            outputs: Set(old.outputs.clone()),
            sla: Set(old.sla.clone()),
            cost_per_transaction: Set(old.cost_per_transaction),
            cycle_time: Set(old.cycle_time),
            automation_level: Set(old.automation_level),
            maturity: Set(old.maturity),
            owner_id: Set(old.owner_id),
            created_by: Set(old.created_by),
            updated_by: Set(old.updated_by),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            space_id: Set(old.space_id),
        };
        let new_model = new_active.insert(&txn).await?;

        txn.commit().await?;

        Ok(PublishVersionResult {
            new_process: new_model.into(),
            affected_links,
        })
    }
}
