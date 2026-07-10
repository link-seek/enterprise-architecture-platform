use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GapAnalysisResult {
    pub gaps: Vec<Gap>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Gap {
    pub area: String,
    pub current: String,
    pub target: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RedundancyResult {
    pub duplicates: Vec<Duplicate>,
    pub mergeable: Vec<Mergeable>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Duplicate {
    pub entity_type: String,
    pub ids: Vec<uuid::Uuid>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Mergeable {
    pub entity_type: String,
    pub source_id: uuid::Uuid,
    pub target_id: uuid::Uuid,
    pub reason: String,
}
