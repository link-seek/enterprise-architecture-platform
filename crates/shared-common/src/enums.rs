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
