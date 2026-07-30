use chrono::Utc;
use shared_common::enums::{SpaceRole, SpaceVisibility, UserRole};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::entity::{Space, SpaceMember};
use crate::domain::space::audit::SpaceAuditLog;
use crate::domain::space::repository::{AuditLogRepository, MembershipRepository, SpaceRepository};

/// Maximum number of spaces a non-admin user may own, including archived ones.
/// Archiving does not release a slot. Admins are unlimited. Fixed per R3/R8
/// (not configurable).
pub const SPACE_QUOTA_LIMIT: u64 = 3;

/// Application Service for Space: orchestrates space CRUD, membership,
/// visibility, and space-level access control (quota + member ACL + visibility).
pub struct SpaceService<S: SpaceRepository, M: MembershipRepository, A: AuditLogRepository> {
    spaces: S,
    members: M,
    audit: A,
}

impl<S: SpaceRepository, M: MembershipRepository, A: AuditLogRepository> SpaceService<S, M, A> {
    pub fn new(spaces: S, members: M, audit: A) -> Self {
        Self { spaces, members, audit }
    }

    /// Create a space. The creator becomes its owner. Non-admin users may own
    /// at most three spaces (including archived); admins are unlimited.
    pub async fn create_space(
        &self,
        creator_id: Uuid,
        creator_role: UserRole,
        name: String,
        description: Option<String>,
        visibility: SpaceVisibility,
    ) -> Result<Space, DomainError> {
        if !creator_role.is_admin() {
            // NOTE: This is a best-effort soft quota. `count_owned_by` and the
            // subsequent `save` + `members.add` are independent async operations
            // with no shared transaction or lock, so concurrent create requests
            // by the same user can all pass this check before any of them
            // inserts. Enforcing a hard limit would require transactional
            // check-and-insert, which the repository traits do not expose; the
            // accepted race is bounded (a user could exceed the limit by at
            // most the number of concurrent requests) and admins remain
            // unlimited.
            let owned = self.spaces.count_owned_by(creator_id).await?;
            if owned >= SPACE_QUOTA_LIMIT {
                return Err(DomainError::SpaceQuotaExceeded);
            }
        }

        let now = Utc::now();
        let id = Uuid::now_v7();
        let space = Space::create(id, name, description, visibility, now)?;
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

    /// Update a space's name/description. Requires owner or admin. Visibility is
    /// intentionally not mutable here — use `set_visibility` (R4).
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

    /// Change a space's visibility. Requires owner or admin. Records an audit
    /// log entry (best-effort: a logging failure is warned and does not block
    /// the visibility change).
    pub async fn set_visibility(
        &self,
        space_id: Uuid,
        actor_id: Uuid,
        actor_role: UserRole,
        visibility: SpaceVisibility,
    ) -> Result<Space, DomainError> {
        self.ensure_can_manage(space_id, actor_id, actor_role).await?;
        let mut space = self.spaces.find_by_id(space_id).await?.ok_or(DomainError::SpaceNotFound)?;
        let from = space.visibility;
        // No-op when the visibility is unchanged: skip the write (and the
        // audit log) to avoid a meaningless database update and an unnecessary
        // bump of `updated_at`.
        if from == visibility {
            return Ok(space);
        }
        let now = Utc::now();
        space.set_visibility(visibility, now);
        let saved = self.spaces.save(&space).await?;

        let log = SpaceAuditLog::visibility_changed(
            Uuid::now_v7(),
            saved.id,
            actor_id,
            from.as_str(),
            visibility.as_str(),
            now,
        );
        if let Err(e) = self.audit.record(&log).await {
            tracing::warn!(
                error = %e,
                space_id = %saved.id,
                actor_id = %actor_id,
                "failed to record space visibility audit log (best-effort)"
            );
        }
        Ok(saved)
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

    /// Spaces visible to an anonymous caller: public non-deleted spaces only.
    pub async fn list_public(&self) -> Result<Vec<Space>, DomainError> {
        self.spaces.find_all_public().await
    }

    /// Fetch a single space by id (regardless of visibility). Callers must
    /// have already passed `ensure_can_read` to authorize the read.
    pub async fn find_space(&self, space_id: Uuid) -> Result<Option<Space>, DomainError> {
        self.spaces.find_by_id(space_id).await
    }

    /// Spaces visible to an authenticated caller: public spaces plus private
    /// spaces they are a member of. Admins see all non-deleted spaces.
    pub async fn list_visible(
        &self,
        actor_id: Uuid,
        actor_role: UserRole,
    ) -> Result<Vec<Space>, DomainError> {
        if actor_role.is_admin() {
            return self.spaces.find_all_non_deleted().await;
        }
        self.spaces.find_visible_for_user(actor_id).await
    }

    /// Visibility-aware read guard. Admins always pass. Public spaces pass for
    /// anyone (including anonymous — callers pass a sentinel id/role for anon).
    /// Private spaces require membership.
    ///
    /// To avoid leaking the existence of private spaces to unauthorized
    /// callers, both "space not found" and "private space the caller cannot
    /// access" collapse to the same `SpaceNotFound` error for non-admins. An
    /// attacker cannot distinguish a non-existent id from a private id they
    /// are not a member of.
    pub async fn ensure_can_read(
        &self,
        space_id: Uuid,
        actor_id: Option<Uuid>,
        actor_role: UserRole,
    ) -> Result<(), DomainError> {
        if actor_role.is_admin() {
            return Ok(());
        }
        let space = self
            .spaces
            .find_by_id(space_id)
            .await?
            .ok_or(DomainError::SpaceNotFound)?;
        if space.visibility.is_public() {
            return Ok(());
        }
        // Private: require membership. Any failure (anonymous caller or
        // non-member) maps to SpaceNotFound so the existence of a private
        // space is not revealed.
        if let Some(actor_id) = actor_id {
            if self.members.find_membership(space_id, actor_id).await?.is_some() {
                return Ok(());
            }
        }
        Err(DomainError::SpaceNotFound)
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
            Ok(self
                .spaces
                .lock()
                .await
                .values()
                .filter(|s| s.deleted_at.is_none() && s.visibility.is_public())
                .cloned()
                .collect())
        }
        async fn find_all_non_deleted(&self) -> Result<Vec<Space>, DomainError> {
            Ok(self
                .spaces
                .lock()
                .await
                .values()
                .filter(|s| s.deleted_at.is_none())
                .cloned()
                .collect())
        }
        async fn find_visible_for_user(&self, user_id: Uuid) -> Result<Vec<Space>, DomainError> {
            let spaces = self.spaces.lock().await;
            let members = self.members.lock().await;
            Ok(spaces
                .values()
                .filter(|s| {
                    if s.deleted_at.is_some() {
                        return false;
                    }
                    if s.visibility.is_public() {
                        return true;
                    }
                    members.contains_key(&(s.id, user_id))
                })
                .cloned()
                .collect())
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
            // Counts all owner memberships (including archived spaces), per R3.
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

    struct FakeAuditRepo {
        logs: tokio::sync::Mutex<Vec<SpaceAuditLog>>,
    }

    impl FakeAuditRepo {
        fn new() -> Self {
            Self { logs: tokio::sync::Mutex::new(Vec::new()) }
        }
    }

    #[async_trait::async_trait]
    impl AuditLogRepository for FakeAuditRepo {
        async fn record(&self, log: &SpaceAuditLog) -> Result<(), DomainError> {
            self.logs.lock().await.push(log.clone());
            Ok(())
        }
    }

    fn svc() -> SpaceService<FakeSpaceRepo, FakeMemberRepo, FakeAuditRepo> {
        let members: MemberStore = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        SpaceService::new(
            FakeSpaceRepo::new(members.clone()),
            FakeMemberRepo::new(members),
            FakeAuditRepo::new(),
        )
    }

    /// Convenience: create a public space (the common case in existing tests).
    async fn create_public(
        s: &SpaceService<FakeSpaceRepo, FakeMemberRepo, FakeAuditRepo>,
        creator_id: Uuid,
        creator_role: UserRole,
        name: &str,
    ) -> Space {
        s.create_space(creator_id, creator_role, name.into(), None, SpaceVisibility::Public)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn create_space_makes_creator_owner() {
        let s = svc();
        let creator = Uuid::now_v7();
        let space = create_public(&s, creator, UserRole::Architect, "S").await;
        let m = s.my_membership(space.id, creator).await.unwrap().unwrap();
        assert_eq!(m.role, SpaceRole::Owner);
    }

    #[tokio::test]
    async fn quota_three_including_archived() {
        let s = svc();
        let user = Uuid::now_v7();
        let s1 = create_public(&s, user, UserRole::Architect, "first").await;
        create_public(&s, user, UserRole::Architect, "second").await;
        create_public(&s, user, UserRole::Architect, "third").await;
        // Fourth is rejected.
        let err = s
            .create_space(user, UserRole::Architect, "fourth".into(), None, SpaceVisibility::Public)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::SpaceQuotaExceeded));
        // Archiving does not release the slot.
        s.archive_space(s1.id, user, UserRole::Architect).await.unwrap();
        let err = s
            .create_space(user, UserRole::Architect, "fifth".into(), None, SpaceVisibility::Public)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::SpaceQuotaExceeded));
    }

    #[tokio::test]
    async fn admin_unlimited_spaces() {
        let s = svc();
        let admin = Uuid::now_v7();
        create_public(&s, admin, UserRole::Admin, "a").await;
        create_public(&s, admin, UserRole::Admin, "b").await;
        create_public(&s, admin, UserRole::Admin, "c").await;
        create_public(&s, admin, UserRole::Admin, "d").await;
    }

    #[tokio::test]
    async fn editor_cannot_manage_members() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let other = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
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
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        let err = s.ensure_can_edit(space.id, stranger, UserRole::Architect).await.unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceMember));
    }

    #[tokio::test]
    async fn editor_can_edit_but_not_manage() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        s.ensure_can_edit(space.id, editor, UserRole::Architect).await.unwrap();
        let err = s.ensure_can_manage(space.id, editor, UserRole::Architect).await.unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceOwner));
    }

    #[tokio::test]
    async fn cannot_remove_last_owner() {
        let s = svc();
        let owner = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        let err = s.remove_member(space.id, owner, UserRole::Architect, owner).await.unwrap_err();
        assert!(matches!(err, DomainError::CannotRemoveLastOwner));
    }

    #[tokio::test]
    async fn cannot_add_duplicate_member() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        let err = s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap_err();
        assert!(matches!(err, DomainError::AlreadyMember));
    }

    #[tokio::test]
    async fn admin_bypasses_membership_checks() {
        let s = svc();
        let owner = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        // admin (non-member) can manage
        s.ensure_can_manage(space.id, admin, UserRole::Admin).await.unwrap();
        s.ensure_can_edit(space.id, admin, UserRole::Admin).await.unwrap();
    }

    #[tokio::test]
    async fn empty_space_name_rejected() {
        let s = svc();
        let user = Uuid::now_v7();
        let err = s
            .create_space(user, UserRole::Architect, "   ".into(), None, SpaceVisibility::Public)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::SpaceNameEmpty));
    }

    // --- Visibility / read-access tests (R2) ---

    #[tokio::test]
    async fn public_space_readable_by_non_member() {
        let s = svc();
        let owner = Uuid::now_v7();
        let stranger = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        // Anonymous (no id) can read public.
        s.ensure_can_read(space.id, None, UserRole::Architect).await.unwrap();
        // Logged-in non-member can read public.
        s.ensure_can_read(space.id, Some(stranger), UserRole::Architect).await.unwrap();
    }

    #[tokio::test]
    async fn private_space_denied_for_non_member() {
        let s = svc();
        let owner = Uuid::now_v7();
        let stranger = Uuid::now_v7();
        let space = s
            .create_space(owner, UserRole::Architect, "S".into(), None, SpaceVisibility::Private)
            .await
            .unwrap();
        // Anonymous denied (existence of private space is not revealed).
        let err = s.ensure_can_read(space.id, None, UserRole::Architect).await.unwrap_err();
        assert!(matches!(err, DomainError::SpaceNotFound));
        // Logged-in non-member denied.
        let err = s.ensure_can_read(space.id, Some(stranger), UserRole::Architect).await.unwrap_err();
        assert!(matches!(err, DomainError::SpaceNotFound));
    }

    #[tokio::test]
    async fn private_space_readable_by_member() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let space = s
            .create_space(owner, UserRole::Architect, "S".into(), None, SpaceVisibility::Private)
            .await
            .unwrap();
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        // Owner (member) can read.
        s.ensure_can_read(space.id, Some(owner), UserRole::Architect).await.unwrap();
        // Editor (member) can read.
        s.ensure_can_read(space.id, Some(editor), UserRole::Architect).await.unwrap();
    }

    #[tokio::test]
    async fn admin_bypasses_private() {
        let s = svc();
        let owner = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let space = s
            .create_space(owner, UserRole::Architect, "S".into(), None, SpaceVisibility::Private)
            .await
            .unwrap();
        // Admin (non-member) can read private.
        s.ensure_can_read(space.id, Some(admin), UserRole::Admin).await.unwrap();
    }

    #[tokio::test]
    async fn set_visibility_requires_owner() {
        let s = svc();
        let owner = Uuid::now_v7();
        let editor = Uuid::now_v7();
        let stranger = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        s.add_member(space.id, owner, UserRole::Architect, editor, SpaceRole::Editor).await.unwrap();
        // Editor cannot change visibility.
        let err = s
            .set_visibility(space.id, editor, UserRole::Architect, SpaceVisibility::Private)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceOwner));
        // Non-member cannot change visibility.
        let err = s
            .set_visibility(space.id, stranger, UserRole::Architect, SpaceVisibility::Private)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotSpaceOwner));
        // Owner can.
        let updated = s
            .set_visibility(space.id, owner, UserRole::Architect, SpaceVisibility::Private)
            .await
            .unwrap();
        assert!(updated.visibility.is_private());
        // Admin can.
        let admin = Uuid::now_v7();
        s.set_visibility(space.id, admin, UserRole::Admin, SpaceVisibility::Public).await.unwrap();
    }

    #[tokio::test]
    async fn set_visibility_noop_when_unchanged() {
        let s = svc();
        let owner = Uuid::now_v7();
        let space = create_public(&s, owner, UserRole::Architect, "S").await;
        let original_updated_at = space.updated_at;
        // Setting to the same visibility is a no-op: the returned space keeps
        // its original updated_at (no save occurred).
        let result = s
            .set_visibility(space.id, owner, UserRole::Architect, SpaceVisibility::Public)
            .await
            .unwrap();
        assert_eq!(result.updated_at, original_updated_at);
        assert!(result.visibility.is_public());
    }
}