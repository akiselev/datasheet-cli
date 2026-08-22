// SPDX-License-Identifier: MIT OR Apache-2.0
//! Durable FDX3 supervision and additive lake-boundary contracts.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CandidateId, EvidenceAnchor, NormalizedQuantity, ParsedQuantityShape, Sha256Digest, SketchId,
};

pub const SOURCE_VOCABULARY_MAP_SCHEMA: &str = "source-vocabulary-map/1";
pub const LABEL_VOTE_SCHEMA: &str = "silver.label_votes/v1";
pub const TRAINING_LABEL_SCHEMA: &str = "silver.training_labels/v1";
pub const STRUCTURED_FACT_SCHEMA: &str = "silver.structured_facts/v1";
pub const TRAINING_EXAMPLE_SCHEMA: &str = "training.examples/v1";
pub const CALIBRATION_SCHEMA: &str = "labeler-calibration/1";
pub const REVIEWED_DEVELOPMENT_SET_SCHEMA: &str = "reviewed-development-set/1";
pub const PRODUCTION_READINESS_SCHEMA: &str = "fdx3-production-readiness/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicyTrainingDecision {
    Allow,
    Deny,
    ReviewRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnalogPowerSuitability {
    EvidenceBearingElectricalCells,
    EvidenceBearingNonElectricalCells,
    StructuredElectricalFacts,
    StructuredNonElectricalFacts,
    NoSuitableEvidence,
}

impl AnalogPowerSuitability {
    #[must_use]
    pub const fn is_suitable(self) -> bool {
        matches!(
            self,
            Self::EvidenceBearingElectricalCells | Self::StructuredElectricalFacts
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductionArtifactBinding {
    pub role: String,
    pub content_sha256: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u64>,
}

/// Fail-closed audit of one real source family considered for analog/power supervision.
/// This record contains identities and decisions only; it must never embed restricted source data.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductionSupervisionFamilyAudit {
    pub family_id: String,
    pub source_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_profile_sha256: Option<Sha256Digest>,
    pub policy_id: String,
    pub policy_version: String,
    pub policy_sha256: Sha256Digest,
    pub policy_committed: bool,
    pub training_decision: PolicyTrainingDecision,
    pub suitability: AnalogPowerSuitability,
    pub artifacts: Vec<ProductionArtifactBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_state_id: Option<String>,
    pub selected_labelers: Vec<LabelerRef>,
    pub source_vocabulary_map_ids: Vec<Sha256Digest>,
    pub reviewed_development_example_ids: Vec<Sha256Digest>,
    pub blocker_codes: Vec<String>,
    pub admitted_for_training: bool,
}

/// Immutable production-readiness selection manifest. Empty map/development/calibration bindings
/// are intentional when rights or evidence preconditions fail; they are not synthetic substitutes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductionReadinessManifest {
    pub schema_version: String,
    pub manifest_digest: Sha256Digest,
    pub audited_at_utc: String,
    pub datasheet_revision: String,
    pub foundry_revision: String,
    pub ferrodoc_revision: String,
    pub fls5_committed: bool,
    pub labeler_catalog_digest: Sha256Digest,
    pub families: Vec<ProductionSupervisionFamilyAudit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_development_set_digest: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_digest: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documented_second_family_blocker: Option<ProductionArtifactBinding>,
    pub eligible_family_count: u32,
    pub production_acceptance: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SupervisionProducer {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    pub configuration_digest: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_digest: Option<Sha256Digest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RightsSnapshot {
    pub snapshot_id: Sha256Digest,
    pub policy_id: String,
    pub policy_version: String,
    pub retention_permitted: bool,
    pub training_permitted: bool,
    pub remote_disclosure_permitted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LineageGroup {
    pub group_id: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_dataset_fingerprint: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation_epoch: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinnedDocumentState {
    pub source_pdf_sha256: Sha256Digest,
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub datasheet_sketch_id: SketchId,
}

/// Exact immutable observation used by one labeler. Document observations must carry their
/// pinned state and anchors; source-native structured facts deliberately do not invent one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SupervisionObservation {
    pub observation_id: Sha256Digest,
    pub source_record_id: Sha256Digest,
    pub source_schema: String,
    pub producer: SupervisionProducer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_state: Option<PinnedDocumentState>,
    pub candidate_ids: Vec<CandidateId>,
    pub evidence_anchors: Vec<EvidenceAnchor>,
    pub lineage_groups: Vec<LineageGroup>,
    pub rights_snapshots: Vec<RightsSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceVocabularyMap {
    pub schema_version: String,
    pub map_id: Sha256Digest,
    pub source_schema: String,
    pub external_namespace: String,
    pub external_field_key: String,
    pub predicate_term_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_quantity_kind: Option<String>,
    pub expected_value_shapes: Vec<ParsedQuantityShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regime_hint: Option<String>,
    pub category_scope: Vec<String>,
    pub mapping_observation_ids: Vec<Sha256Digest>,
    pub ontology_snapshot_digest: Sha256Digest,
    pub producer: SupervisionProducer,
    pub mapping_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelerRef {
    pub labeler_id: String,
    pub labeler_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum VoteOutcome {
    Vote { vote_json: Value },
    Abstain { abstain_reason: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelVote {
    pub schema_version: String,
    /// Datasheet semantic identity; Foundry assigns a distinct physical lake `record_id`.
    pub vote_id: Sha256Digest,
    pub training_candidate_id: CandidateId,
    pub task: String,
    pub labeler: LabelerRef,
    pub outcome: VoteOutcome,
    pub observations: Vec<SupervisionObservation>,
    /// Exact union of the groups carried by `observations`, retained for physical lake projection.
    pub lineage_group_ids: Vec<Sha256Digest>,
    pub reason_codes: Vec<String>,
    pub producer: SupervisionProducer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SupervisionClass {
    DeterministicHigh,
    WeakConsensus,
    TeacherReviewed,
    HumanGold,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SupervisionUncertainty {
    pub posterior_millionths: u32,
    pub disagreement_millionths: u32,
    pub independent_lineage_count: u32,
    pub competing_target_count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrainingLabel {
    pub schema_version: String,
    /// Datasheet semantic identity; Foundry assigns a distinct physical lake `record_id`.
    pub training_label_id: Sha256Digest,
    pub training_candidate_id: CandidateId,
    pub task: String,
    pub target_json: Value,
    pub supervision_class: SupervisionClass,
    pub uncertainty: SupervisionUncertainty,
    pub input_vote_record_ids: Vec<Sha256Digest>,
    pub combiner_id: String,
    pub combiner_version: String,
    pub combiner_configuration_digest: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_record_id: Option<Sha256Digest>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StructuredFactRecord {
    pub schema_version: String,
    /// Datasheet semantic identity; Foundry assigns a distinct physical lake `record_id`.
    pub structured_fact_id: Sha256Digest,
    pub bronze_fact_record_id: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_vocabulary_map_id: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicate_candidate_id: Option<CandidateId>,
    pub raw_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_quantity: Option<NormalizedQuantity>,
    pub normalization_status: String,
    pub producer: SupervisionProducer,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrainingExample {
    pub schema_version: String,
    pub example_id: Sha256Digest,
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_state_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasheet_sketch_digest: Option<Sha256Digest>,
    pub observation_ids: Vec<Sha256Digest>,
    pub source_record_ids: Vec<Sha256Digest>,
    pub candidate_ids: Vec<CandidateId>,
    pub training_label_record_id: Sha256Digest,
    pub document_family_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_family_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_key: Option<String>,
    pub assigned_split: String,
    pub ontology_snapshot_digest: Sha256Digest,
    pub rights_snapshot_ids: Vec<Sha256Digest>,
    pub producer: SupervisionProducer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelerCalibration {
    pub labeler: LabelerRef,
    pub reviewed_correct: u32,
    pub reviewed_total: u32,
    pub precision_millionths: u32,
    pub weight_millionths: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalibrationSnapshot {
    pub schema_version: String,
    pub digest: Sha256Digest,
    pub reviewed_development_set_digest: Sha256Digest,
    pub ontology_snapshot_digest: Sha256Digest,
    pub entries: Vec<LabelerCalibration>,
    pub minimum_posterior_millionths: u32,
    pub minimum_independent_lineages: u32,
    pub producer: SupervisionProducer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewedDevelopmentExample {
    pub example_id: Sha256Digest,
    pub task: String,
    pub training_candidate_id: CandidateId,
    pub expected_target_json: Value,
    pub observation_ids: Vec<Sha256Digest>,
    pub reviewer_record_id: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewedDevelopmentSet {
    pub schema_version: String,
    pub digest: Sha256Digest,
    pub ontology_snapshot_digest: Sha256Digest,
    pub examples: Vec<ReviewedDevelopmentExample>,
    pub producer: SupervisionProducer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupervisionContractError {
    Duplicate(&'static str),
    Invalid(&'static str),
    Missing(&'static str),
}

impl std::fmt::Display for SupervisionContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SupervisionContractError {}

impl SupervisionProducer {
    pub fn validate(&self) -> Result<(), SupervisionContractError> {
        if self.name.trim().is_empty()
            || self.version.trim().is_empty()
            || self
                .build
                .as_ref()
                .is_some_and(|build| build.trim().is_empty())
        {
            return Err(SupervisionContractError::Missing("producer identity"));
        }
        self.configuration_digest
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("producer configuration digest"))?;
        if let Some(model) = &self.model_digest {
            model
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("producer model digest"))?;
        }
        Ok(())
    }
}

impl RightsSnapshot {
    pub fn validate(&self) -> Result<(), SupervisionContractError> {
        self.snapshot_id
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("rights snapshot digest"))?;
        if self.policy_id.trim().is_empty() || self.policy_version.trim().is_empty() {
            return Err(SupervisionContractError::Missing("rights policy identity"));
        }
        if !self.retention_permitted
            && (self.training_permitted || self.remote_disclosure_permitted)
        {
            return Err(SupervisionContractError::Invalid(
                "non-retained input cannot be trained on or disclosed",
            ));
        }
        Ok(())
    }
}

impl LineageGroup {
    #[must_use]
    pub fn recompute_id(&self) -> Sha256Digest {
        #[derive(Serialize)]
        struct View<'a> {
            publisher_key: &'a Option<String>,
            upstream_dataset_fingerprint: &'a Option<Sha256Digest>,
            source_schema: &'a Option<String>,
            observation_epoch: &'a Option<String>,
        }
        Sha256Digest::of_bytes(
            &serde_json::to_vec(&View {
                publisher_key: &self.publisher_key,
                upstream_dataset_fingerprint: &self.upstream_dataset_fingerprint,
                source_schema: &self.source_schema,
                observation_epoch: &self.observation_epoch,
            })
            .expect("lineage identity serialization cannot fail"),
        )
    }

    pub fn validate(&self) -> Result<(), SupervisionContractError> {
        self.group_id
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("lineage group digest"))?;
        if self
            .publisher_key
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            && self.upstream_dataset_fingerprint.is_none()
            && self
                .source_schema
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            && self
                .observation_epoch
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(SupervisionContractError::Missing("lineage group identity"));
        }
        if let Some(digest) = &self.upstream_dataset_fingerprint {
            digest
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("upstream dataset digest"))?;
        }
        if self.group_id != self.recompute_id() {
            return Err(SupervisionContractError::Invalid(
                "lineage group content identity",
            ));
        }
        Ok(())
    }
}

impl SupervisionObservation {
    pub fn validate(&self) -> Result<(), SupervisionContractError> {
        self.observation_id
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("observation digest"))?;
        self.source_record_id
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("source record digest"))?;
        self.producer.validate()?;
        if self.source_schema.trim().is_empty()
            || self.lineage_groups.is_empty()
            || self.rights_snapshots.is_empty()
        {
            return Err(SupervisionContractError::Missing(
                "observation schema, lineage, or rights",
            ));
        }
        unique(
            self.lineage_groups.iter().map(|group| &group.group_id),
            "lineage group",
        )?;
        unique(
            self.rights_snapshots
                .iter()
                .map(|rights| &rights.snapshot_id),
            "rights snapshot",
        )?;
        unique(self.candidate_ids.iter(), "observation candidate")?;
        for group in &self.lineage_groups {
            group.validate()?;
        }
        for rights in &self.rights_snapshots {
            rights.validate()?;
        }
        match &self.document_state {
            Some(state) => {
                state
                    .source_pdf_sha256
                    .validate()
                    .map_err(|_| SupervisionContractError::Invalid("document source PDF digest"))?;
                if state.document_state_id.trim().is_empty() || self.evidence_anchors.is_empty() {
                    return Err(SupervisionContractError::Missing(
                        "document state or evidence anchors",
                    ));
                }
                if let Some(digest) = &state.document_ir_logical_sha256 {
                    digest.validate().map_err(|_| {
                        SupervisionContractError::Invalid("document IR logical digest")
                    })?;
                }
                for anchor in &self.evidence_anchors {
                    anchor
                        .validate()
                        .map_err(|_| SupervisionContractError::Invalid("evidence anchor"))?;
                    if anchor.source_pdf_sha256 != state.source_pdf_sha256
                        || anchor.document_state_id != state.document_state_id
                        || anchor.document_ir_logical_sha256 != state.document_ir_logical_sha256
                    {
                        return Err(SupervisionContractError::Invalid(
                            "anchor is not from the pinned document state",
                        ));
                    }
                }
            }
            None if !self.candidate_ids.is_empty() || !self.evidence_anchors.is_empty() => {
                return Err(SupervisionContractError::Invalid(
                    "document candidates or anchors require a pinned state",
                ));
            }
            None => {}
        }
        Ok(())
    }

    #[must_use]
    pub fn permits_training(&self) -> bool {
        !self.rights_snapshots.is_empty()
            && self
                .rights_snapshots
                .iter()
                .all(|rights| rights.retention_permitted && rights.training_permitted)
    }
}

impl LabelVote {
    pub fn validate_structure(&self) -> Result<(), SupervisionContractError> {
        if self.schema_version != LABEL_VOTE_SCHEMA
            || self.training_candidate_id.0.trim().is_empty()
            || self.task.trim().is_empty()
            || self.labeler.labeler_id.trim().is_empty()
            || self.labeler.labeler_version.trim().is_empty()
            || self.observations.is_empty()
            || self.reason_codes.is_empty()
            || self.reason_codes.iter().any(|code| code.trim().is_empty())
        {
            return Err(SupervisionContractError::Missing("vote identity or reason"));
        }
        self.vote_id
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("vote record digest"))?;
        self.producer.validate()?;
        unique(
            self.observations.iter().map(|item| &item.observation_id),
            "observation",
        )?;
        unique(self.lineage_group_ids.iter(), "lineage group ID")?;
        unique(self.reason_codes.iter(), "reason code")?;
        let mut actual = self
            .observations
            .iter()
            .flat_map(|item| {
                item.lineage_groups
                    .iter()
                    .map(|group| group.group_id.clone())
            })
            .collect::<Vec<_>>();
        actual.sort();
        actual.dedup();
        if actual != self.lineage_group_ids {
            return Err(SupervisionContractError::Invalid(
                "vote lineage projection does not match observations",
            ));
        }
        for observation in &self.observations {
            observation.validate()?;
        }
        match &self.outcome {
            VoteOutcome::Vote { vote_json } if vote_json.is_null() => {
                Err(SupervisionContractError::Invalid("null vote"))
            }
            VoteOutcome::Abstain { abstain_reason } if abstain_reason.trim().is_empty() => {
                Err(SupervisionContractError::Missing("abstention reason"))
            }
            _ => Ok(()),
        }
    }
}

impl CalibrationSnapshot {
    pub fn validate_structure(&self) -> Result<(), SupervisionContractError> {
        if self.schema_version != CALIBRATION_SCHEMA
            || self.entries.is_empty()
            || self.minimum_posterior_millionths > 1_000_000
            || self.minimum_independent_lineages == 0
        {
            return Err(SupervisionContractError::Invalid("calibration bounds"));
        }
        for digest in [
            &self.digest,
            &self.reviewed_development_set_digest,
            &self.ontology_snapshot_digest,
        ] {
            digest
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("calibration digest"))?;
        }
        self.producer.validate()?;
        unique(
            self.entries
                .iter()
                .map(|entry| (&entry.labeler.labeler_id, &entry.labeler.labeler_version)),
            "calibration labeler",
        )?;
        for entry in &self.entries {
            if entry.labeler.labeler_id.trim().is_empty()
                || entry.labeler.labeler_version.trim().is_empty()
                || entry.reviewed_total == 0
                || entry.reviewed_correct > entry.reviewed_total
                || entry.precision_millionths > 1_000_000
                || entry.weight_millionths == 0
            {
                return Err(SupervisionContractError::Invalid("calibration entry"));
            }
            let measured_precision = u64::from(entry.reviewed_correct) * 1_000_000
                / u64::from(entry.reviewed_total);
            if u64::from(entry.precision_millionths) != measured_precision {
                return Err(SupervisionContractError::Invalid(
                    "calibration precision does not match review counts",
                ));
            }
        }
        Ok(())
    }
}

impl StructuredFactRecord {
    pub fn validate_structure(&self) -> Result<(), SupervisionContractError> {
        if self.schema_version != STRUCTURED_FACT_SCHEMA
            || self.raw_value.trim().is_empty()
            || self.normalization_status.trim().is_empty()
            || self
                .warnings
                .iter()
                .any(|warning| warning.trim().is_empty())
        {
            return Err(SupervisionContractError::Missing(
                "structured fact value or normalization status",
            ));
        }
        for digest in [
            Some(&self.structured_fact_id),
            Some(&self.bronze_fact_record_id),
            self.source_vocabulary_map_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            digest
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("structured fact digest"))?;
        }
        self.producer.validate()?;
        unique(self.warnings.iter(), "structured fact warning")
    }
}

impl TrainingExample {
    pub fn validate_structure(&self) -> Result<(), SupervisionContractError> {
        const SPLITS: [&str; 6] = [
            "train",
            "development",
            "template-held-out",
            "document-family-held-out",
            "manufacturer-held-out",
            "final-blind-test",
        ];
        if self.schema_version != TRAINING_EXAMPLE_SCHEMA
            || self.task.trim().is_empty()
            || self.candidate_ids.is_empty()
            || self.document_family_id.trim().is_empty()
            || !SPLITS.contains(&self.assigned_split.as_str())
            || self.rights_snapshot_ids.is_empty()
        {
            return Err(SupervisionContractError::Missing(
                "training example state, family, split, candidates, or rights",
            ));
        }
        let document_backed = self
            .document_state_id
            .as_deref()
            .is_some_and(|state| !state.trim().is_empty());
        let source_backed = !self.observation_ids.is_empty() && !self.source_record_ids.is_empty();
        if document_backed == source_backed
            || (document_backed && self.datasheet_sketch_digest.is_none())
            || (!document_backed
                && (self.document_ir_logical_sha256.is_some()
                    || self.datasheet_sketch_digest.is_some()))
        {
            return Err(SupervisionContractError::Invalid(
                "training example must have exactly one document or source-native identity",
            ));
        }
        if (self.assigned_split == "template-held-out"
            && self
                .template_family_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty()))
            || (self.assigned_split == "manufacturer-held-out"
                && self
                    .manufacturer_key
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty()))
        {
            return Err(SupervisionContractError::Missing("held-out grouping key"));
        }
        for digest in [
            Some(&self.example_id),
            self.document_ir_logical_sha256.as_ref(),
            self.datasheet_sketch_digest.as_ref(),
            Some(&self.training_label_record_id),
            Some(&self.ontology_snapshot_digest),
        ]
        .into_iter()
        .flatten()
        {
            digest
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("training example digest"))?;
        }
        self.producer.validate()?;
        unique(self.candidate_ids.iter(), "training candidate")?;
        unique(self.observation_ids.iter(), "training observation")?;
        unique(self.source_record_ids.iter(), "training source record")?;
        unique(self.rights_snapshot_ids.iter(), "training rights snapshot")
    }
}

impl ProductionArtifactBinding {
    fn validate(&self) -> Result<(), SupervisionContractError> {
        if self.role.trim().is_empty()
            || self
                .logical_id
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || self
                .schema_id
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SupervisionContractError::Missing(
                "production artifact identity",
            ));
        }
        self.content_sha256
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("production artifact digest"))
    }
}

impl ProductionSupervisionFamilyAudit {
    pub fn validate(&self) -> Result<(), SupervisionContractError> {
        if self.family_id.trim().is_empty()
            || self.source_key.trim().is_empty()
            || self.policy_id.trim().is_empty()
            || self.policy_version.trim().is_empty()
            || self.artifacts.is_empty()
            || self.selected_labelers.is_empty()
            || self.blocker_codes.iter().any(|code| code.trim().is_empty())
        {
            return Err(SupervisionContractError::Missing(
                "production family audit identity",
            ));
        }
        for digest in [
            Some(&self.policy_sha256),
            self.source_profile_sha256.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            digest
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("source audit digest"))?;
        }
        if self.source_profile_id.is_some() != self.source_profile_sha256.is_some() {
            return Err(SupervisionContractError::Invalid(
                "source profile ID/digest binding",
            ));
        }
        unique(
            self.artifacts.iter().map(|item| &item.role),
            "artifact role",
        )?;
        unique(
            self.selected_labelers
                .iter()
                .map(|item| (&item.labeler_id, &item.labeler_version)),
            "selected labeler",
        )?;
        unique(
            self.source_vocabulary_map_ids.iter(),
            "source vocabulary map",
        )?;
        unique(
            self.reviewed_development_example_ids.iter(),
            "development example",
        )?;
        unique(self.blocker_codes.iter(), "blocker code")?;
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        for digest in self
            .source_vocabulary_map_ids
            .iter()
            .chain(&self.reviewed_development_example_ids)
        {
            digest.validate().map_err(|_| {
                SupervisionContractError::Invalid("production supervision binding digest")
            })?;
        }
        if self.selected_labelers.iter().any(|labeler| {
            labeler.labeler_id.trim().is_empty() || labeler.labeler_version.trim().is_empty()
        }) {
            return Err(SupervisionContractError::Missing(
                "selected labeler identity",
            ));
        }
        let eligible = self.policy_committed
            && self.training_decision == PolicyTrainingDecision::Allow
            && self.suitability.is_suitable()
            && self.blocker_codes.is_empty()
            && (!matches!(
                self.suitability,
                AnalogPowerSuitability::EvidenceBearingElectricalCells
            ) || self
                .document_state_id
                .as_deref()
                .is_some_and(|state| !state.trim().is_empty()));
        if self.admitted_for_training != eligible {
            return Err(SupervisionContractError::Invalid(
                "fail-closed production admission decision",
            ));
        }
        Ok(())
    }
}

impl ProductionReadinessManifest {
    #[must_use]
    pub fn recompute_digest(&self) -> Sha256Digest {
        #[derive(Serialize)]
        struct View<'a> {
            schema_version: &'a str,
            audited_at_utc: &'a str,
            datasheet_revision: &'a str,
            foundry_revision: &'a str,
            ferrodoc_revision: &'a str,
            fls5_committed: bool,
            labeler_catalog_digest: &'a Sha256Digest,
            families: &'a [ProductionSupervisionFamilyAudit],
            reviewed_development_set_digest: &'a Option<Sha256Digest>,
            calibration_digest: &'a Option<Sha256Digest>,
            documented_second_family_blocker: &'a Option<ProductionArtifactBinding>,
            eligible_family_count: u32,
            production_acceptance: bool,
        }
        let mut families = self.families.clone();
        for family in &mut families {
            family
                .artifacts
                .sort_by(|left, right| left.role.cmp(&right.role));
            family.selected_labelers.sort_by(|left, right| {
                (&left.labeler_id, &left.labeler_version)
                    .cmp(&(&right.labeler_id, &right.labeler_version))
            });
            family.source_vocabulary_map_ids.sort();
            family.reviewed_development_example_ids.sort();
            family.blocker_codes.sort();
        }
        families.sort_by(|left, right| left.family_id.cmp(&right.family_id));
        Sha256Digest::of_bytes(
            &serde_json::to_vec(&View {
                schema_version: &self.schema_version,
                audited_at_utc: &self.audited_at_utc,
                datasheet_revision: &self.datasheet_revision,
                foundry_revision: &self.foundry_revision,
                ferrodoc_revision: &self.ferrodoc_revision,
                fls5_committed: self.fls5_committed,
                labeler_catalog_digest: &self.labeler_catalog_digest,
                families: &families,
                reviewed_development_set_digest: &self.reviewed_development_set_digest,
                calibration_digest: &self.calibration_digest,
                documented_second_family_blocker: &self.documented_second_family_blocker,
                eligible_family_count: self.eligible_family_count,
                production_acceptance: self.production_acceptance,
            })
            .expect("readiness identity serialization cannot fail"),
        )
    }

    pub fn validate(&self) -> Result<(), SupervisionContractError> {
        if self.schema_version != PRODUCTION_READINESS_SCHEMA
            || self.audited_at_utc.trim().is_empty()
            || self.datasheet_revision.trim().is_empty()
            || self.foundry_revision.trim().is_empty()
            || self.ferrodoc_revision.trim().is_empty()
            || self.families.is_empty()
        {
            return Err(SupervisionContractError::Missing(
                "production readiness identity",
            ));
        }
        self.manifest_digest
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("readiness manifest digest"))?;
        self.labeler_catalog_digest
            .validate()
            .map_err(|_| SupervisionContractError::Invalid("labeler catalog digest"))?;
        unique(
            self.families.iter().map(|family| &family.family_id),
            "family",
        )?;
        for family in &self.families {
            family.validate()?;
        }
        for digest in [
            self.reviewed_development_set_digest.as_ref(),
            self.calibration_digest.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            digest
                .validate()
                .map_err(|_| SupervisionContractError::Invalid("development/calibration digest"))?;
        }
        if self.calibration_digest.is_some() && self.reviewed_development_set_digest.is_none() {
            return Err(SupervisionContractError::Invalid(
                "calibration requires a reviewed development set",
            ));
        }
        if let Some(blocker) = &self.documented_second_family_blocker {
            blocker.validate()?;
            if blocker.role != "documented_second_family_access_or_rights_blocker" {
                return Err(SupervisionContractError::Invalid(
                    "second-family blocker role",
                ));
            }
        }
        let eligible = u32::try_from(
            self.families
                .iter()
                .filter(|family| family.admitted_for_training)
                .count(),
        )
        .map_err(|_| SupervisionContractError::Invalid("eligible family count"))?;
        if eligible != self.eligible_family_count {
            return Err(SupervisionContractError::Invalid("eligible family count"));
        }
        let family_requirement =
            eligible >= 2 || (eligible == 1 && self.documented_second_family_blocker.is_some());
        let acceptance = self.fls5_committed
            && family_requirement
            && self.reviewed_development_set_digest.is_some()
            && self.calibration_digest.is_some()
            && self
                .families
                .iter()
                .filter(|family| family.admitted_for_training)
                .all(|family| {
                    !family.source_vocabulary_map_ids.is_empty()
                        && !family.reviewed_development_example_ids.is_empty()
                });
        if acceptance != self.production_acceptance
            || self.manifest_digest != self.recompute_digest()
        {
            return Err(SupervisionContractError::Invalid(
                "production readiness content identity or acceptance",
            ));
        }
        Ok(())
    }
}

fn unique<T: Ord>(
    values: impl IntoIterator<Item = T>,
    what: &'static str,
) -> Result<(), SupervisionContractError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(SupervisionContractError::Duplicate(what));
        }
    }
    Ok(())
}
