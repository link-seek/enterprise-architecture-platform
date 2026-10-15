//! Unified publish engine (discussions#752 unified publish policy v1).
//!
//! Every publishable element in this platform (value streams, business
//! processes, …) publishes through [`publish`] — exactly one implementation
//! of the flow. Per-entity differences are data mapping only (which tables
//! hold content children and associations) and live in [`PublishStrategy`]
//! impls. Everything policy-shaped is defined here exactly once:
//!
//! 1. strict semver validation of the requested version,
//! 2. monotonicity against every existing version of the logical id
//!    (no duplicates, no rollbacks),
//! 3. Active→Deprecated compare-and-swap (concurrent double-publish loses
//!    instead of forking a second Active row),
//! 4. new Active row insert + content children copy,
//! 5. association MOVE (single UPDATEs, never INSERT+DELETE),
//! 6. impact receipt of the moved links.
//!
//! All six steps run inside a single transaction.
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};
use semver::Version;
use uuid::Uuid;

use super::version;
use crate::domain::error::DomainError;

/// One version-anchored association moved to the new version row by a
/// publish (UI impact receipt). Both value streams and business processes
/// anchor capabilities, so the receipt shape is shared.
#[derive(Debug, Clone)]
pub struct MovedLink {
    pub capability_id: Uuid,
    pub capability_name: String,
    pub old_version: String,
    pub new_version: String,
}

/// Caller-supplied content overrides for the new version row.
/// Value streams allow renaming on publish; processes always copy verbatim.
#[derive(Debug, Clone, Default)]
pub struct PublishOverrides {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Publish strategy: the ONLY per-entity surface of the unified engine.
/// Impls only map data; policy lives in [`publish`].
#[async_trait::async_trait]
pub trait PublishStrategy: Send + Sync {
    /// Human entity name for errors ("BusinessProcess" / "ValueStream").
    fn entity_name(&self) -> &'static str;

    /// Live database handle (the engine opens the transaction on it).
    fn db(&self) -> &DatabaseConnection;

    /// All `(row id, business_version)` of non-deleted rows for `logical_id`.
    async fn existing_versions(
        &self,
        txn: &DatabaseTransaction,
        logical_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, DomainError>;

    /// CAS-flip the active row to Deprecated (compatibility window).
    /// Returns `(old row id, old version)`. Must fail when no active row
    /// exists (concurrent double-publish loses here instead of forking).
    async fn deprecate_active(
        &self,
        txn: &DatabaseTransaction,
        logical_id: Uuid,
    ) -> Result<(Uuid, String), DomainError>;

    /// Insert the new Active row (copying content + children from `old_id`).
    /// Returns the new row id.
    async fn insert_new(
        &self,
        txn: &DatabaseTransaction,
        old_id: Uuid,
        new_version: &str,
        overrides: &PublishOverrides,
    ) -> Result<Uuid, DomainError>;

    /// MOVE version-anchored associations old→new (single UPDATEs, never
    /// INSERT+DELETE). Returns the moved links for the impact receipt.
    async fn move_links(
        &self,
        txn: &DatabaseTransaction,
        old_id: Uuid,
        new_id: Uuid,
        old_version: &str,
        new_version: &str,
    ) -> Result<Vec<MovedLink>, DomainError>;
}

/// Publish one new version. See module docs for the six unified steps.
pub async fn publish(
    strategy: &impl PublishStrategy,
    logical_id: Uuid,
    new_version: &str,
    overrides: &PublishOverrides,
) -> Result<(Uuid, Vec<MovedLink>), DomainError> {
    // Step 1: strict semver, same rule for every element.
    let new_parsed = version::parse(new_version).map_err(DomainError::from)?;
    let new_version = new_version.to_owned();
    let txn = strategy.db().begin().await.map_err(|e| {
        DomainError::Database(format!("{} publish: begin transaction: {e}", strategy.entity_name()))
    })?;

    // Step 2: monotonicity against the MAXIMUM existing version, so the
    // `from` in the error is deterministic. Unparseable history rows are
    // data corruption, not skippable: fail loudly.
    let existing = strategy.existing_versions(&txn, logical_id).await
        .map_err(|e| {
            DomainError::Database(format!(
                "{} publish: list existing versions: {e}",
                strategy.entity_name()
            ))
        })?;
    let mut max: Option<Version> = None;
    let mut max_raw = String::new();
    for (_id, ver) in &existing {
        let v = version::parse(ver).map_err(|e| {
            DomainError::Validation(format!("stored version '{ver}' is not valid semver: {e}"))
        })?;
        if max.as_ref().map_or(true, |m| v > *m) {
            max = Some(v);
            max_raw = ver.clone();
        }
    }
    if let Some(m) = max {
        if new_parsed <= m {
            return Err(DomainError::InvalidTransition {
                from: max_raw,
                to: new_version.clone(),
                entity: strategy.entity_name().to_string(),
            });
        }
    }

    // Steps 3–5: flip, insert, move — one transaction.
    let (old_id, old_version) = strategy.deprecate_active(&txn, logical_id).await?;
    let new_id = strategy
        .insert_new(&txn, old_id, &new_version, overrides)
        .await?;
    let moved = strategy
        .move_links(&txn, old_id, new_id, &old_version, &new_version)
        .await?;

    txn.commit().await.map_err(|e| {
        DomainError::Database(format!("{} publish: commit transaction: {e}", strategy.entity_name()))
    })?;
    Ok((new_id, moved))
}
