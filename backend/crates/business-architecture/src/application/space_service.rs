use chrono::Utc;
use shared_common::enums::{SpaceRole, UserRole};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::entity::{Space, SpaceMember};
use crate::domain::space::repository::{MembershipRepository, SpaceRepository};

/// Application Service for Space: orchestrates space CRUD, membership, and
/// space-level access control (quota + member ACL).
pub struct SpaceService<S: SpaceRepository, M: MembershipRepository> {
    spaces: S,
    members: M,
}

impl<S: SpaceRepository, M: MembershipRepository> SpaceService<S, M> {
    pub fn new(spaces: S, members: M) -> Self {
        Self { spaces, members }
    }

    /// Create a space. The creator becomes its owner. Non-admin users may own
    /// at most one space (quota); admins are unlimited.
    pub async fn create_space(
        &self,
        creator_id: Uuid,
        creator_role: UserRole,
        name: String,
        description: Option<String>,
    ) -> Result<Space, DomainError> {
        if !creator_role.is_admin() {
            let owned = self.spaces.count_owned_by(creator_id).await?;
            if owned >= 1 {
                return Err(DomainError::SpaceQuotaExceeded);
            }
        }

        let now = Utc::now();
        let id = Uuid::now_v7();
        let space = Space::create(id, name, description, now)?;
        let saved = self.spaces.save(&space).await?;

        // Creator becomes owner.
        let member = SpaceMember {
            space_id: saved.id,
            user_id: creator_id,
            role: SpaceRole::Owner,
            created_at: now,
            updated_at: now,
        };
        self.members.add(&member).await?;
        Ok(saved)
    }

    /// Update a space's name/description. Requires owner or admin.
    pub async fn update_space(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        actor_role: UserRole,
        name: Option<String>,
        description: Option<Option<String>>,
    ) -> Result<Space, DomainError> {
        self.ensure_can_manage(space_id, actor_id, actor_role).await?;
        let mut space = self.spaces.find_by_id(space_id).await?.ok_or(DomainError::SpaceNotFound)?;
        let now = Utc::now();
        if let Some(n) = name {
            space.rename(n, now)?;
        }
        if let Some(d) = description {
            space.set_description(d, now);
        }
        self.spaces.save(&space).await
    }

    /// Soft-delete (archive) a space. Requires owner or admin.
    pub async fn archive_space(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        actor_role: UserRole,
    ) -> Result<(), DomainError> {
        self.ensure_can_manage(space_id, actor_id, actor_role).await?;
        self.spaces.soft_delete(space_id).await
    }

    /// List all public (non-deleted) spaces.
    pub async fn list_public(&self) -> Result<Vec<Space>, DomainError> {
        self.spaces.find_all_public().await
    }

    /// Add a member to a space. Requires owner or admin. Prevents duplicates.
    pub async fn add_member(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        actor_role: UserRole,
        user_id: Uuid,
        role: SpaceRole,
    ) -> Result<SpaceMember, DomainError> {
        self.ensure_can_manage(space_id, actor_id, actor_role).await?;
        if self.members.find_membership(space_id, user_id).await?.is_some() {
            return Err(DomainError::AlreadyMember);
        }
        let now = Utc::now();
        let member = SpaceMember {
            space_id,
            user_id,
            role,
            created_at: now,
            updated_at: now,
        };
        self.members.add(&member).await
    }

    /// Remove a member. Requires owner or admin. Prevents removing the last owner.
    pub async fn remove_member(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        actor_role: UserRole,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        self.ensure_can_manage(space_id, actor_id, actor_role).await?;
        let target = self.members.find_membership(space_id, user_id).await?;
        if let Some(m) = &target {
            if m.role.is_owner() {
                let owners = self.members.count_owners(space_id).await?;
                if owners <= 1 {
                    return Err(DomainError::CannotRemoveLastOwner);
                }
            }
        } else {
            return Err(DomainError::NotSpaceMember);
        }
        self.members.remove(space_id, user_id).await
    }

    pub async fn list_members(&self, space_id: Uuid) -> Result<Vec<SpaceMember>, DomainError> {
        self.members.list_members(space_id).await
    }

    /// Membership of a specific user in a space (for frontend edit-permission checks).
    pub async fn my_membership(&self, space_id: Uuid, user_id: Uuid) -> Result<Option<SpaceMember>, DomainError> {
        self.members.find_membership(space_id, user_id).await
    }

    /// Ensure the actor may edit content in the space (editor or owner, or admin).
    pub async fn ensure_can_edit(&self, space_id: Uuid, actor_id: Uuid, actor_role: UserRole) -> Result<(), DomainError> {
        if actor_role.is_admin() {
            return Ok(());
        }
        let m = self.members.find_membership(space_id, actor_id).await?
            .ok_or(DomainError::NotSpaceMember)?;
        if !m.role.is_editor() {
            return Err(DomainError::NotSpaceEditor);
        }
        Ok(())
    }

    /// Ensure the actor may manage the space (owner or admin).
    pub async fn ensure_can_manage(&self, space_id: Uuid, actor_id: Uuid, actor_role: UserRole) -> Result<(), DomainError> {
        if actor_role.is_admin() {
            return Ok(());
        }
        let m = self.members.find_membership(space_id, actor_id).await?
            .ok_or(DomainError::NotSpaceOwner)?;
        if !m.role.is_owner() {
            return Err(DomainError::NotSpaceOwner);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    // In-memory fakes for the repository traits. They share the membership
    // store so `count_owned_by` can count owner memberships the way the real
    // SeaORM repo does. These exercise the *real* SpaceService authorization
    // logic against a controllable store — they are not mocks of the unit
    // under test.
    type MemberStore = Arc<tokio::sync::Mutex<HashMap<(Uuid, Uuid), SpaceMember>>>;

    struct FakeSpaceRepo {
        spaces: tokio::sync::Mutex<HashMap<Uuid, Space>>,
        members: MemberStore,
    }

    impl FakeSpaceRepo {
        fn new(members: MemberStore) -> Self {
            Self { spaces: tokio::sync::Mutex::new(HashMap::new()), members }
        }
    }

    #[async_trait::async_trait]
    impl SpaceRepository for FakeSpaceRepo {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<Space>, DomainError> {
            Ok(self.spaces.lock().await.get(&id).cloned())
        }
        async fn find_all_public(&self) -> Result<Vec<Space>, DomainError> {
            Ok(self.spaces.lock().await.values().filter(|s| s.deleted_at.is_none()).cloned().collect())
        }
        async fn save(&self, space: &Space) -> Result<Space, DomainError> {
            self.spaces.lock().await.insert(space.id, space.clone());
            Ok(space.clone())
        }
        async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
            let mut g = self.spaces.lock().await;
            if let Some(s) = g.get_mut(&id) {
                s.archive(Utc::now());
            }
            Ok(())
        }
        async fn count_owned_by(&self, user_id: Uuid) -> Result<u64, DomainError> {
            Ok(self.members.lock().await.values().filter(|m| m.user_id == user_id && m.role.is_owner()).count() as u64)
        }
    }

    struct FakeMemberRepo {
        members: MemberStore,
    }

    impl FakeMemberRepo {
        fn new(members: MemberStore) -> Self {
            Self { members }
        }
    }

    #[async_trait::async_trait]
    impl MembershipRepository for FakeMemberRepo {
        async fn find_membership(&self, space_id: Uuid, user_id: Uuid) -> Result<Option<SpaceMember>, DomainError> {
            Ok(self.members.lock().await.get(&(space_id, user_id)).cloned())
        }
        async fn list_members(&self, space_id: Uuid) -> Result<Vec<SpaceMember>, DomainError> {
            Ok(self.members.lock().await.values().filter(|m| m.space_id == space_id).cloned().collect())
        }
        async fn add(&self, member: &SpaceMember) -> Result<SpaceMember, DomainError> {
            self.members.lock().await.insert((member.space_id, member.user_id), member.clone());
            Ok(member.clone())
        }
        async fn remove(&self, space_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
            self.members.lock().await.remove(&(space_id, user_id));
            Ok(())
        }
        async fn count_owners(&self, space_id: Uuid) -> Result<u64, DomainError> {
            Ok(self.members.lock().await.values().filter(|m| m.space_id == space_id && m.role.is_owner()).count() as u64)
        }
    }

    fn svc() -> SpaceService<FakeSpaceRepo, FakeMemberRepo> {
        let members: MemberStore = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        SpaceService::new(FakeSpaceRepo::new(members.clone()), FakeMemberRepo::new(members))
    }

    #[tokio::test]
    async fn create_space_makes_creator_owner() {
        let s = svc();
        let creator = Uuid::now_v7();
        let space = s.create_space(creator, UserRole::Architect, "S".into(), None).await.unwrap();
        let m = s.my_membership(space.id, creator).await.unwrap().unwrap();
        assert_eq!(m.role, SpaceRole::Owner);
    }

    #[tokio::test]
    async fn non_admin_quota_one_space() {
        let s = svc();
        let user = Uuid::now_v7();
        s.create_space(user, UserRole::Architect, "first".into(), None).await.unwrap();
        let err = s.create_space(user, UserRole::Architect, "second".into(), None).await.unwrap_err();
        assert!(matches!(err, DomainError::SpaceQuotaExceeded));
    }

    #[tokio::test]
    async fn admin_unlimited_spaces() {
        let s = svc();
        let admin = Uuid::now_v7();
        s.create_space(admin, UserRole::Admin, "a".into(), None).await.unwrap();
        s.create_space(admin, UserRole::Admin, "b".into(), None).await.unwrap();
    }

    #[tokio::test]
    async fn editor_cannot_manage_members() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let other = Uuid::now_v7();
        let space = s.create_space(owner, UserRole::Architect, "S".into(), None).await.unwrap();
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        // editor attempting to add a member → denied
        let err = s.add_member(space.id, editor, UserRole::Architect, other, SpaceRole::Editor).await.unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceOwner));
    }

    #[tokio::test]
    async fn non_member_cannot_edit() {
        let s = svc();
        let owner = Uuid::now_v7();
        let stranger = Uuid::now_v7();
        let space = s.create_space(owner, UserRole::Architect, "S".into(), None).await.unwrap();
        let err = s.ensure_can_edit(space.id, stranger, UserRole::Architect).await.unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceMember));
    }

    #[tokio::test]
    async fn editor_can_edit_but_not_manage() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let space = s.create_space(owner, UserRole::Architect, "S".into(), None).await.unwrap();
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        s.ensure_can_edit(space.id, editor, UserRole::Architect).await.unwrap();
        let err = s.ensure_can_manage(space.id, editor, UserRole::Architect).await.unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceOwner));
    }

    #[tokio::test]
    async fn cannot_remove_last_owner() {
        let s = svc();
        let owner = Uuid::now_v7();
        let space = s.create_space(owner, UserRole::Architect, "S".into(), None).await.unwrap();
        let err = s.remove_member(space.id, owner, UserRole::Architect, owner).await.unwrap_err();
        assert!(matches!(err, DomainError::CannotRemoveLastOwner));
    }

    #[tokio::test]
    async fn cannot_add_duplicate_member() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let space = s.create_space(owner, UserRole::Architect, "S".into(), None).await.unwrap();
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        let err = s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap_err();
        assert!(matches!(err, DomainError::AlreadyMember));
    }

    #[tokio::test]
    async fn admin_bypasses_membership_checks() {
        let s = svc();
        let owner = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let space = s.create_space(owner, UserRole::Architect, "S".into(), None).await.unwrap();
        // admin (non-member) can manage
        s.ensure_can_manage(space.id, admin, UserRole::Admin).await.unwrap();
        s.ensure_can_edit(space.id, admin, UserRole::Admin).await.unwrap();
    }

    #[tokio::test]
    async fn empty_space_name_rejected() {
        let s = svc();
        let user = Uuid::now_v7();
        let err = s.create_space(user, UserRole::Architect, "   ".into(), None).await.unwrap_err();
        assert!(matches!(err, DomainError::SpaceNameEmpty));
    }
}