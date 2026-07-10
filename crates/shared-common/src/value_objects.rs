use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NaturalId(pub String);

impl std::fmt::Display for NaturalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct StringVec(pub Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct NaturalIdVec(pub Vec<NaturalId>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, FromJsonQueryResult)]
pub struct StringStringMap(pub std::collections::HashMap<String, String>);
