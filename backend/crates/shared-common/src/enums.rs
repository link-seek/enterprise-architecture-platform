use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLevel {
    #[sea_orm(string_value = "l1")]
    #[serde(rename = "l1")]
    L1,
    #[sea_orm(string_value = "l2")]
    #[serde(rename = "l2")]
    L2,
    #[sea_orm(string_value = "l3")]
    #[serde(rename = "l3")]
    L3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum AutomationLevel {
    #[sea_orm(string_value = "manual")]
    #[serde(rename = "manual")]
    Manual,
    #[sea_orm(string_value = "semi_automated")]
    #[serde(rename = "semi_automated")]
    SemiAutomated,
    #[sea_orm(string_value = "fully_automated")]
    #[serde(rename = "fully_automated")]
    FullyAutomated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum MaturityLevel {
    #[sea_orm(string_value = "level1")]
    #[serde(rename = "level1")]
    Level1,
    #[sea_orm(string_value = "level2")]
    #[serde(rename = "level2")]
    Level2,
    #[sea_orm(string_value = "level3")]
    #[serde(rename = "level3")]
    Level3,
    #[sea_orm(string_value = "level4")]
    #[serde(rename = "level4")]
    Level4,
    #[sea_orm(string_value = "level5")]
    #[serde(rename = "level5")]
    Level5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum BusinessValueRating {
    #[sea_orm(string_value = "high")]
    #[serde(rename = "high")]
    High,
    #[sea_orm(string_value = "medium")]
    #[serde(rename = "medium")]
    Medium,
    #[sea_orm(string_value = "low")]
    #[serde(rename = "low")]
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum CostRating {
    #[sea_orm(string_value = "high")]
    #[serde(rename = "high")]
    High,
    #[sea_orm(string_value = "medium")]
    #[serde(rename = "medium")]
    Medium,
    #[sea_orm(string_value = "low")]
    #[serde(rename = "low")]
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    #[sea_orm(string_value = "active")]
    #[serde(rename = "active")]
    Active,
    #[sea_orm(string_value = "archived")]
    #[serde(rename = "archived")]
    Archived,
}

/// Tracks the operational state of a business capability.
///
/// Stored in the `capability_status` column of `business_capabilities`
/// (the `status` column is already used by [`LifecycleStatus`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    #[sea_orm(string_value = "active")]
    #[serde(rename = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    #[serde(rename = "inactive")]
    Inactive,
    #[sea_orm(string_value = "draft")]
    #[serde(rename = "draft")]
    Draft,
}

impl Default for CapabilityStatus {
    fn default() -> Self {
        CapabilityStatus::Active
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum ValueStreamImportance {
    #[sea_orm(string_value = "critical")]
    #[serde(rename = "critical")]
    Critical,
    #[sea_orm(string_value = "high")]
    #[serde(rename = "high")]
    High,
    #[sea_orm(string_value = "medium")]
    #[serde(rename = "medium")]
    Medium,
    #[sea_orm(string_value = "low")]
    #[serde(rename = "low")]
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    #[sea_orm(string_value = "admin")]
    #[serde(rename = "admin")]
    Admin,
    #[sea_orm(string_value = "architect")]
    #[serde(rename = "architect")]
    Architect,
    #[sea_orm(string_value = "viewer")]
    #[serde(rename = "viewer")]
    Viewer,
}

impl UserRole {
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
    pub fn is_architect(&self) -> bool {
        matches!(self, UserRole::Architect)
    }
    pub fn is_viewer(&self) -> bool {
        matches!(self, UserRole::Viewer)
    }
    pub fn can_create(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Architect)
    }
    pub fn can_update(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Architect)
    }
    pub fn can_delete(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Architect)
    }
    pub fn can_use_ai(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Architect)
    }
    pub fn can_manage_users(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
    pub fn can_transfer_owner(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(UserRole::Admin),
            "architect" => Some(UserRole::Architect),
            "viewer" => Some(UserRole::Viewer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    #[sea_orm(string_value = "active")]
    #[serde(rename = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    #[serde(rename = "inactive")]
    Inactive,
    #[sea_orm(string_value = "banned")]
    #[serde(rename = "banned")]
    Banned,
}

impl UserStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, UserStatus::Active)
    }
}

/// Role of a user within a Space (multi-tenant membership).
///
/// - `Owner`: created the space (or was transferred ownership). Can manage
///   members and delete the space. Implies editor rights.
/// - `Editor`: can create/update/delete architecture content within the space.
///
/// Anonymous users and non-members have read-only access (enforced at the
/// GraphQL layer), so there is no explicit `Viewer` member role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum SpaceRole {
    #[sea_orm(string_value = "owner")]
    #[serde(rename = "owner")]
    Owner,
    #[sea_orm(string_value = "editor")]
    #[serde(rename = "editor")]
    Editor,
}

impl SpaceRole {
    pub fn is_owner(&self) -> bool {
        matches!(self, SpaceRole::Owner)
    }
    pub fn is_editor(&self) -> bool {
        matches!(self, SpaceRole::Owner | SpaceRole::Editor)
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(SpaceRole::Owner),
            "editor" => Some(SpaceRole::Editor),
            _ => None,
        }
    }
}

/// Visibility of a Space (multi-tenant access scope).
///
/// - `Public`: content readable by anonymous users and any logged-in user.
/// - `Private`: content readable only by space members (owner/editor) and
///   platform Admins.
///
/// The database column default is `'public'` (see migration 000031) so existing
/// spaces remain open with zero behavior change when the column is added. The
/// Rust `Default` trait, however, returns `Private` to match the least-privilege
/// default used by `parse_visibility_arg`, preventing accidental public exposure
/// when visibility is derived via `Default` rather than specified explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum SpaceVisibility {
    #[sea_orm(string_value = "public")]
    Public,
    #[sea_orm(string_value = "private")]
    Private,
}

impl Default for SpaceVisibility {
    fn default() -> Self {
        SpaceVisibility::Private
    }
}

impl SpaceVisibility {
    pub fn is_private(&self) -> bool {
        matches!(self, SpaceVisibility::Private)
    }
    pub fn is_public(&self) -> bool {
        matches!(self, SpaceVisibility::Public)
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceVisibility::Public => "public",
            SpaceVisibility::Private => "private",
        }
    }
}

impl std::str::FromStr for SpaceVisibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(SpaceVisibility::Public),
            "private" => Ok(SpaceVisibility::Private),
            _ => Err(format!("invalid SpaceVisibility: '{s}'")),
        }
    }
}

/// Categorises an application component by its architectural role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum ApplicationComponentType {
    #[sea_orm(string_value = "workflow")]
    Workflow,
    #[sea_orm(string_value = "script")]
    Script,
    #[sea_orm(string_value = "service")]
    Service,
    #[sea_orm(string_value = "ui")]
    Ui,
}

/// Lifecycle state of an application component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum ApplicationComponentStatus {
    #[sea_orm(string_value = "draft")]
    Draft,
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "deprecated")]
    Deprecated,
}

/// How an application process is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum ApplicationProcessTrigger {
    #[sea_orm(string_value = "push")]
    Push,
    #[sea_orm(string_value = "pull_request")]
    PullRequest,
    #[sea_orm(string_value = "schedule")]
    Schedule,
    #[sea_orm(string_value = "manual")]
    Manual,
    #[sea_orm(string_value = "webhook")]
    Webhook,
}

/// RACI role for Participation (BusinessRole → BusinessProcess).
///
/// Maps to the RACI matrix: Responsible (does the work), Accountable (owns
/// the outcome), Consulted (provides input), Informed (kept up to date).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum RaciRole {
    #[sea_orm(string_value = "responsible")]
    Responsible,
    #[sea_orm(string_value = "accountable")]
    Accountable,
    #[sea_orm(string_value = "consulted")]
    Consulted,
    #[sea_orm(string_value = "informed")]
    Informed,
}

/// Type of an organizational unit (ArchiMate Business Actor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum OrganizationalUnitType {
    #[sea_orm(string_value = "team")]
    Team,
    #[sea_orm(string_value = "role")]
    Role,
    #[sea_orm(string_value = "unit")]
    Unit,
    #[sea_orm(string_value = "external")]
    External,
}

/// Lifecycle state of a functional module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum FunctionalModuleStatus {
    #[sea_orm(string_value = "draft")]
    Draft,
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "deprecated")]
    Deprecated,
}

/// Protocol of an application interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum ApplicationInterfaceProtocol {
    #[sea_orm(string_value = "workflow_dispatch")]
    WorkflowDispatch,
    #[sea_orm(string_value = "api")]
    Api,
    #[sea_orm(string_value = "webhook")]
    Webhook,
}

/// Which kind of process a CapabilityRealization targets.
///
/// After the v2.1 redesign, a capability is enabled by a *process* (business
/// or application) rather than by an application component. This enum
/// discriminates the polymorphic target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, DeriveActiveEnum, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRealizationTargetType {
    #[sea_orm(string_value = "business_process")]
    BusinessProcess,
    #[sea_orm(string_value = "application_process")]
    ApplicationProcess,
}
