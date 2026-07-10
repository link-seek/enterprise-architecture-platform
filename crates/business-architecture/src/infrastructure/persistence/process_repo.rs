use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, TransactionTrait,
};
use shared_common::enums::LifecycleStatus;
use uuid::Uuid;

use crate::application::version::bump_minor;
use crate::domain::error::DomainError;
use crate::domain::process::entity::{BusinessProcess, ProcessStep};
use crate::domain::process::repository::{ProcessRepository, ProcessStepRepository};
use crate::infrastructure::persistence::entities::{business_process, process_step};

impl From<business_process::Model> for BusinessProcess {
    fn from(m: business_process::Model) -> Self {
        BusinessProcess {
            id: m.id,
            logical_id: m.logical_id,
            business_version: m.business_version,
            status: m.status,
            name: m.name,
            description: m.description,
            sla: m.sla,
            cost_per_transaction: m.cost_per_transaction,
            cycle_time: m.cycle_time,
            owner_id: m.owner_id,
            created_by: m.created_by,
            updated_by: m.updated_by,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
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
            active.sla = Set(proc.sla.clone());
            active.cost_per_transaction = Set(proc.cost_per_transaction);
            active.cycle_time = Set(proc.cycle_time);
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
                sla: Set(proc.sla.clone()),
                cost_per_transaction: Set(proc.cost_per_transaction),
                cycle_time: Set(proc.cycle_time),
                owner_id: Set(proc.owner_id),
                created_by: Set(proc.created_by),
                updated_by: Set(proc.updated_by),
                created_at: Set(proc.created_at),
                updated_at: Set(proc.updated_at),
                deleted_at: Set(None),
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

        let mut active: business_process::ActiveModel = model.into();
        active.status = Set(LifecycleStatus::Archived);
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
    pub async fn publish_new_version(
        &self,
        logical_id: Uuid,
    ) -> Result<BusinessProcess, DomainError> {
        let txn = self.db.begin().await?;

        let old = business_process::Entity::find()
            .filter(business_process::Column::LogicalId.eq(logical_id))
            .filter(business_process::Column::Status.eq(LifecycleStatus::Active))
            .filter(business_process::Column::DeletedAt.is_null())
            .one(&txn)
            .await?
            .ok_or(DomainError::ProcessVersionNotFound)?;

        let new_version = bump_minor(&old.business_version)?;

        let mut old_active: business_process::ActiveModel = old.clone().into();
        old_active.status = Set(LifecycleStatus::Archived);
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
            sla: Set(old.sla.clone()),
            cost_per_transaction: Set(old.cost_per_transaction),
            cycle_time: Set(old.cycle_time),
            owner_id: Set(old.owner_id),
            created_by: Set(old.created_by),
            updated_by: Set(old.updated_by),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        };
        let new_model = new_active.insert(&txn).await?;

        txn.commit().await?;

        Ok(new_model.into())
    }
}
