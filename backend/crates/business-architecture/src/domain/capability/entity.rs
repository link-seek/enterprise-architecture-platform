use chrono::{DateTime, Utc};
use shared_common::enums::{
    BusinessValueRating, CapabilityLevel, CapabilityStatus, CostRating, LifecycleStatus,
    MaturityLevel,
};
use shared_common::value_objects::StringStringMap;
use uuid::Uuid;

use super::super::error::DomainError;

#[derive(Debug, Clone)]
pub struct BusinessCapability {
    pub id: Uuid,
    pub logical_id: Uuid,
    pub status: LifecycleStatus,
    pub capability_status: CapabilityStatus,
    pub name: String,
    pub description: String,
    pub level: CapabilityLevel,
    pub maturity: MaturityLevel,
    pub business_value: BusinessValueRating,
    pub cost: CostRating,
    pub metrics: Option<StringStringMap>,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub space_id: Uuid,
}

impl BusinessCapability {
    /// Archive this capability. Only active capabilities can be archived.
    /// Lifecycle state transition: Active → Archived (one-way, value-stream
    /// style). Capabilities carry no version chain, so no Deprecated
    /// compatibility window is needed; existing `link_process` guards already
    /// reject new relations to non-active capabilities.
    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != LifecycleStatus::Active {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Archived".to_string(),
                entity: "BusinessCapability".to_string(),
            });
        }
        self.status = LifecycleStatus::Archived;
        self.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active_capability() -> BusinessCapability {
        let now = Utc::now();
        BusinessCapability {
            id: Uuid::now_v7(),
            logical_id: Uuid::now_v7(),
            status: LifecycleStatus::Active,
            capability_status: CapabilityStatus::default(),
            name: "test".to_string(),
            description: String::new(),
            level: CapabilityLevel::L1,
            maturity: MaturityLevel::Level3,
            business_value: BusinessValueRating::Medium,
            cost: CostRating::Medium,
            metrics: None,
            owner_id: None,
            created_by: None,
            updated_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            space_id: Uuid::now_v7(),
        }
    }

    #[test]
    fn archive_active_capability() {
        let mut cap = active_capability();
        assert!(cap.archive(Utc::now()).is_ok());
        assert_eq!(cap.status, LifecycleStatus::Archived);
    }

    #[test]
    fn archive_non_active_capability_fails() {
        let mut cap = active_capability();
        cap.status = LifecycleStatus::Archived;
        assert!(cap.archive(Utc::now()).is_err());
    }
}
