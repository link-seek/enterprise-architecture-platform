use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use shared_common::enums::LifecycleStatus;
use uuid::Uuid;

use crate::domain::capability::entity::BusinessCapability;
use crate::domain::capability::repository::CapabilityRepository;
use crate::domain::error::DomainError;
use crate::infrastructure::persistence::entities::{business_capability, capability_process};

impl From<business_capability::Model> for BusinessCapability {
    fn from(m: business_capability::Model) -> Self {
        BusinessCapability {
            id: m.id,
            business_version: m.business_version,
            status: m.status,
            name: m.name,
            description: m.description,
            level: m.level,
            maturity: m.maturity,
            business_value: m.business_value,
            cost: m.cost,
            owner_id: m.owner_id,
            created_by: m.created_by,
            updated_by: m.updated_by,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }
    }
}

pub struct SeaOrmCapabilityRepo {
    db: DatabaseConnection,
}

impl SeaOrmCapabilityRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CapabilityRepository for SeaOrmCapabilityRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<BusinessCapability>, DomainError> {
        let model = business_capability::Entity::find()
            .filter(business_capability::Column::Id.eq(id))
            .filter(business_capability::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn save(&self, cap: &BusinessCapability) -> Result<BusinessCapability, DomainError> {
        let existing = business_capability::Entity::find_by_id(cap.id)
            .one(&self.db)
            .await?;

        let result = if let Some(model) = existing {
            let mut active: business_capability::ActiveModel = model.into();
            active.business_version = Set(cap.business_version.clone());
            active.status = Set(cap.status);
            active.name = Set(cap.name.clone());
            active.description = Set(cap.description.clone());
            active.level = Set(cap.level);
            active.maturity = Set(cap.maturity);
            active.business_value = Set(cap.business_value);
            active.cost = Set(cap.cost);
            active.owner_id = Set(cap.owner_id);
            active.updated_by = Set(cap.updated_by);
            active.updated_at = Set(cap.updated_at);
            active.update(&self.db).await?
        } else {
            let active = business_capability::ActiveModel {
                id: Set(cap.id),
                business_version: Set(cap.business_version.clone()),
                status: Set(cap.status),
                name: Set(cap.name.clone()),
                description: Set(cap.description.clone()),
                level: Set(cap.level),
                maturity: Set(cap.maturity),
                business_value: Set(cap.business_value),
                cost: Set(cap.cost),
                owner_id: Set(cap.owner_id),
                created_by: Set(cap.created_by),
                updated_by: Set(cap.updated_by),
                created_at: Set(cap.created_at),
                updated_at: Set(cap.updated_at),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = business_capability::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::CapabilityNotFound)?;

        let mut active: business_capability::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }

    async fn list_active(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<BusinessCapability>, u64), DomainError> {
        let paginator = business_capability::Entity::find()
            .filter(business_capability::Column::DeletedAt.is_null())
            .filter(business_capability::Column::Status.eq(LifecycleStatus::Active))
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let models = paginator.fetch_page(page.saturating_sub(1)).await?;

        let caps = models.into_iter().map(Into::into).collect();
        Ok((caps, total))
    }
}

impl SeaOrmCapabilityRepo {
    pub async fn find_processes(
        &self,
        capability_id: Uuid,
    ) -> Result<Vec<Uuid>, DomainError> {
        let links = capability_process::Entity::find()
            .filter(capability_process::Column::CapabilityId.eq(capability_id))
            .all(&self.db)
            .await?;
        Ok(links.into_iter().map(|l| l.process_id).collect())
    }

    pub async fn link_process(
        &self,
        capability_id: Uuid,
        process_id: Uuid,
    ) -> Result<(), DomainError> {
        let process = crate::infrastructure::persistence::entities::business_process::Entity::find()
            .filter(crate::infrastructure::persistence::entities::business_process::Column::Id.eq(process_id))
            .filter(crate::infrastructure::persistence::entities::business_process::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?
            .ok_or(DomainError::ProcessNotFound)?;

        if process.status != LifecycleStatus::Active {
            return Err(DomainError::CannotReferenceArchived);
        }

        let active = capability_process::ActiveModel {
            capability_id: Set(capability_id),
            process_id: Set(process_id),
        };
        capability_process::Entity::insert(active)
            .on_conflict(sea_orm::sea_query::OnConflict::new().do_nothing().to_owned())
            .exec(&self.db)
            .await?;

        Ok(())
    }

    pub async fn unlink_process(
        &self,
        capability_id: Uuid,
        process_id: Uuid,
    ) -> Result<(), DomainError> {
        capability_process::Entity::delete_many()
            .filter(capability_process::Column::CapabilityId.eq(capability_id))
            .filter(capability_process::Column::ProcessId.eq(process_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
