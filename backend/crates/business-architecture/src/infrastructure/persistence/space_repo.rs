use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use shared_common::enums::SpaceRole;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::entity::{Space, SpaceMember};
use crate::domain::space::repository::{MembershipRepository, SpaceRepository};
use crate::infrastructure::persistence::entities::{space, space_member};

impl From<space::Model> for Space {
    fn from(m: space::Model) -> Self {
        Space {
            id: m.id,
            name: m.name,
            description: m.description,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }
    }
}

impl From<space_member::Model> for SpaceMember {
    fn from(m: space_member::Model) -> Self {
        SpaceMember {
            space_id: m.space_id,
            user_id: m.user_id,
            role: SpaceRole::from_str(&m.role).unwrap_or(SpaceRole::Editor),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

pub struct SeaOrmSpaceRepo {
    db: DatabaseConnection,
}

impl SeaOrmSpaceRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SpaceRepository for SeaOrmSpaceRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Space>, DomainError> {
        let model = space::Entity::find()
            .filter(space::Column::Id.eq(id))
            .filter(space::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn find_all_public(&self) -> Result<Vec<Space>, DomainError> {
        let models = space::Entity::find()
            .filter(space::Column::DeletedAt.is_null())
            .order_by_asc(space::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn save(&self, space_obj: &Space) -> Result<Space, DomainError> {
        let existing = space::Entity::find_by_id(space_obj.id).one(&self.db).await?;
        let result = if let Some(model) = existing {
            let mut active: space::ActiveModel = model.into();
            active.name = Set(space_obj.name.clone());
            active.description = Set(space_obj.description.clone());
            active.updated_at = Set(space_obj.updated_at);
            active.deleted_at = Set(space_obj.deleted_at);
            active.update(&self.db).await?
        } else {
            let active = space::ActiveModel {
                id: Set(space_obj.id),
                name: Set(space_obj.name.clone()),
                description: Set(space_obj.description.clone()),
                created_at: Set(space_obj.created_at),
                updated_at: Set(space_obj.updated_at),
                deleted_at: Set(space_obj.deleted_at),
            };
            active.insert(&self.db).await?
        };
        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now();
        space::Entity::update_many()
            .col_expr(space::Column::DeletedAt, sea_orm::sea_query::Expr::value(now))
            .col_expr(space::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
            .filter(space::Column::Id.eq(id))
            .filter(space::Column::DeletedAt.is_null())
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn count_owned_by(&self, user_id: Uuid) -> Result<u64, DomainError> {
        // Count owner memberships only for non-archived spaces. Without this
        // filter a non-admin who archives their only space would remain at the
        // 1-space quota and could never create another.
        let pairs = space_member::Entity::find()
            .filter(space_member::Column::UserId.eq(user_id))
            .filter(space_member::Column::Role.eq("owner"))
            .find_also_related(space::Entity)
            .all(&self.db)
            .await?;
        let count = pairs
            .into_iter()
            .filter(|(_, space_opt)| {
                space_opt
                    .as_ref()
                    .map(|s| s.deleted_at.is_none())
                    .unwrap_or(false)
            })
            .count() as u64;
        Ok(count)
    }
}

pub struct SeaOrmMembershipRepo {
    db: DatabaseConnection,
}

impl SeaOrmMembershipRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MembershipRepository for SeaOrmMembershipRepo {
    async fn find_membership(&self, space_id: Uuid, user_id: Uuid) -> Result<Option<SpaceMember>, DomainError> {
        let model = space_member::Entity::find()
            .filter(space_member::Column::SpaceId.eq(space_id))
            .filter(space_member::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn list_members(&self, space_id: Uuid) -> Result<Vec<SpaceMember>, DomainError> {
        let models = space_member::Entity::find()
            .filter(space_member::Column::SpaceId.eq(space_id))
            .order_by_asc(space_member::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn add(&self, member: &SpaceMember) -> Result<SpaceMember, DomainError> {
        let role_str = match member.role {
            SpaceRole::Owner => "owner".to_owned(),
            SpaceRole::Editor => "editor".to_owned(),
        };
        let existing = space_member::Entity::find_by_id((member.space_id, member.user_id))
            .one(&self.db)
            .await?;
        let result = if let Some(model) = existing {
            let mut active: space_member::ActiveModel = model.into();
            active.role = Set(role_str);
            active.updated_at = Set(member.updated_at);
            active.update(&self.db).await?
        } else {
            let active = space_member::ActiveModel {
                space_id: Set(member.space_id),
                user_id: Set(member.user_id),
                role: Set(role_str),
                created_at: Set(member.created_at),
                updated_at: Set(member.updated_at),
            };
            active.insert(&self.db).await?
        };
        Ok(result.into())
    }

    async fn remove(&self, space_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        space_member::Entity::delete_many()
            .filter(space_member::Column::SpaceId.eq(space_id))
            .filter(space_member::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn count_owners(&self, space_id: Uuid) -> Result<u64, DomainError> {
        let count = space_member::Entity::find()
            .filter(space_member::Column::SpaceId.eq(space_id))
            .filter(space_member::Column::Role.eq("owner"))
            .count(&self.db)
            .await?;
        Ok(count)
    }
}