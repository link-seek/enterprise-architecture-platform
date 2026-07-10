use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_common::enums::{
    BusinessValueRating, CapabilityLevel, CostRating, LifecycleStatus, MaturityLevel,
    ValueStreamImportance,
};
use shared_common::value_objects::{StringStringMap, StringVec};
use uuid::Uuid;

use crate::domain::capability::entity::BusinessCapability;
use crate::domain::process::entity::{BusinessProcess, ProcessStep};
use crate::domain::value_stream::entity::{ValueStream, ValueStreamStage};

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCapabilityInput {
    pub name: String,
    pub description: String,
    pub level: CapabilityLevel,
    pub maturity: MaturityLevel,
    pub business_value: BusinessValueRating,
    pub cost: CostRating,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCapabilityInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub level: Option<CapabilityLevel>,
    pub maturity: Option<MaturityLevel>,
    pub business_value: Option<BusinessValueRating>,
    pub cost: Option<CostRating>,
    pub owner_id: Option<Option<Uuid>>,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityDto {
    pub id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub level: CapabilityLevel,
    pub maturity: MaturityLevel,
    pub business_value: BusinessValueRating,
    pub cost: CostRating,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BusinessCapability> for CapabilityDto {
    fn from(c: BusinessCapability) -> Self {
        CapabilityDto {
            id: c.id,
            business_version: c.business_version,
            status: c.status,
            name: c.name,
            description: c.description,
            level: c.level,
            maturity: c.maturity,
            business_value: c.business_value,
            cost: c.cost,
            owner_id: c.owner_id,
            created_by: c.created_by,
            updated_by: c.updated_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProcessInput {
    pub name: String,
    pub description: String,
    pub sla: Option<String>,
    pub cost_per_transaction: Option<f64>,
    pub cycle_time: Option<i64>,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProcessInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sla: Option<Option<String>>,
    pub cost_per_transaction: Option<Option<f64>>,
    pub cycle_time: Option<Option<i64>>,
    pub owner_id: Option<Option<Uuid>>,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessDto {
    pub id: Uuid,
    pub logical_id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub sla: Option<String>,
    pub cost_per_transaction: Option<f64>,
    pub cycle_time: Option<i64>,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BusinessProcess> for ProcessDto {
    fn from(p: BusinessProcess) -> Self {
        ProcessDto {
            id: p.id,
            logical_id: p.logical_id,
            business_version: p.business_version,
            status: p.status,
            name: p.name,
            description: p.description,
            sla: p.sla,
            cost_per_transaction: p.cost_per_transaction,
            cycle_time: p.cycle_time,
            owner_id: p.owner_id,
            created_by: p.created_by,
            updated_by: p.updated_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProcessStepInput {
    pub name: String,
    pub description: String,
    pub sequence_order: i32,
    pub business_rules: StringVec,
    pub required_inputs: StringVec,
    pub produced_outputs: StringVec,
    pub role_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessStepDto {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub sequence_order: i32,
    pub business_rules: StringVec,
    pub required_inputs: StringVec,
    pub produced_outputs: StringVec,
    pub role_id: Option<Uuid>,
    pub process_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProcessStep> for ProcessStepDto {
    fn from(s: ProcessStep) -> Self {
        ProcessStepDto {
            id: s.id,
            name: s.name,
            description: s.description,
            sequence_order: s.sequence_order,
            business_rules: s.business_rules,
            required_inputs: s.required_inputs,
            produced_outputs: s.produced_outputs,
            role_id: s.role_id,
            process_id: s.process_id,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateValueStreamInput {
    pub name: String,
    pub description: String,
    pub triggering_event: Option<String>,
    pub end_deliverable: Option<String>,
    pub owner_id: Option<Uuid>,
    pub importance: ValueStreamImportance,
    pub stakeholders: StringVec,
    pub performance_metrics: StringStringMap,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateValueStreamInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub triggering_event: Option<Option<String>>,
    pub end_deliverable: Option<Option<String>>,
    pub owner_id: Option<Option<Uuid>>,
    pub importance: Option<ValueStreamImportance>,
    pub stakeholders: Option<StringVec>,
    pub performance_metrics: Option<StringStringMap>,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueStreamDto {
    pub id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub triggering_event: Option<String>,
    pub end_deliverable: Option<String>,
    pub owner_id: Option<Uuid>,
    pub importance: ValueStreamImportance,
    pub stakeholders: StringVec,
    pub performance_metrics: StringStringMap,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ValueStream> for ValueStreamDto {
    fn from(v: ValueStream) -> Self {
        ValueStreamDto {
            id: v.id,
            business_version: v.business_version,
            status: v.status,
            name: v.name,
            description: v.description,
            triggering_event: v.triggering_event,
            end_deliverable: v.end_deliverable,
            owner_id: v.owner_id,
            importance: v.importance,
            stakeholders: v.stakeholders,
            performance_metrics: v.performance_metrics,
            created_by: v.created_by,
            updated_by: v.updated_by,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateValueStreamStageInput {
    pub name: String,
    pub sequence_order: i32,
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueStreamStageDto {
    pub id: Uuid,
    pub name: String,
    pub sequence_order: i32,
    pub input: Option<String>,
    pub output: Option<String>,
    pub value_stream_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ValueStreamStage> for ValueStreamStageDto {
    fn from(s: ValueStreamStage) -> Self {
        ValueStreamStageDto {
            id: s.id,
            name: s.name,
            sequence_order: s.sequence_order,
            input: s.input,
            output: s.output,
            value_stream_id: s.value_stream_id,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkProcessInput {
    pub process_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkStageCapabilityInput {
    pub capability_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GapAnalysisInput {
    pub target_maturity: MaturityLevel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedundancyInput {
    pub threshold: Option<f64>,
}
