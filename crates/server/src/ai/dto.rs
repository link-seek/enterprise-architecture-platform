use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiScenario {
    CapabilityDecomposition,
    ProcessDesign,
    GapAnalysis,
    RedundancyIdentification,
}

impl std::fmt::Display for AiScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiScenario::CapabilityDecomposition => write!(f, "capability_decomposition"),
            AiScenario::ProcessDesign => write!(f, "process_design"),
            AiScenario::GapAnalysis => write!(f, "gap_analysis"),
            AiScenario::RedundancyIdentification => write!(f, "redundancy_identification"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiSuggestion {
    pub entity_type: String,
    pub action: String,
    pub data: serde_json::Value,
    pub confidence: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiResponse {
    pub scenario: String,
    pub suggestions: Vec<AiSuggestion>,
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AiRequest {
    pub scenario: AiScenario,
    pub context: serde_json::Value,
}
