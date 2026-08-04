use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct NaturalId(pub String);

impl std::fmt::Display for NaturalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult, ToSchema)]
pub struct StringVec(pub Vec<String>);

impl StringVec {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult, ToSchema)]
pub struct NaturalIdVec(pub Vec<NaturalId>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult, ToSchema)]
pub struct StringStringMap(pub std::collections::HashMap<String, String>);

impl StringStringMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
