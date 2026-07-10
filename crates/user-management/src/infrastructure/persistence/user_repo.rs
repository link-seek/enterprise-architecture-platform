use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::user::entity::User;
use crate::domain::user::repository::UserRepository;
use crate::infrastructure::persistence::entities::user;

impl From<user::Model> for User {
    fn from(m: user::Model) -> Self {
        User {
            id: m.id,
            email: m.email,
            name: m.name,
            password_hash: m.password_hash,
            role: m.role,
            status: m.status,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

pub struct SeaOrmUserRepo {
    db: DatabaseConnection,
}

impl SeaOrmUserRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let model = user::Entity::find()
            .filter(user::Column::Id.eq(id))
            .filter(user::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError> {
        let model = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn save(&self, user_entity: &User) -> Result<User, DomainError> {
        let existing = user::Entity::find_by_id(user_entity.id)
            .one(&self.db)
            .await?;

        let result = if let Some(model) = existing {
            let mut active: user::ActiveModel = model.into();
            active.email = Set(user_entity.email.clone());
            active.name = Set(user_entity.name.clone());
            active.password_hash = Set(user_entity.password_hash.clone());
            active.role = Set(user_entity.role);
            active.status = Set(user_entity.status);
            active.updated_at = Set(user_entity.updated_at);
            active.update(&self.db).await?
        } else {
            let active = user::ActiveModel {
                id: Set(user_entity.id),
                email: Set(user_entity.email.clone()),
                name: Set(user_entity.name.clone()),
                password_hash: Set(user_entity.password_hash.clone()),
                role: Set(user_entity.role),
                status: Set(user_entity.status),
                created_at: Set(user_entity.created_at),
                updated_at: Set(user_entity.updated_at),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = user::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        let mut active: user::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }

    async fn list(&self, page: u64, per_page: u64) -> Result<(Vec<User>, u64), DomainError> {
        let paginator = user::Entity::find()
            .filter(user::Column::DeletedAt.is_null())
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let models = paginator.fetch_page(page.saturating_sub(1)).await?;

        let users = models.into_iter().map(Into::into).collect();
        Ok((users, total))
    }
}
