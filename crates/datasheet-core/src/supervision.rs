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
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub datasheet_sketch_digest: Sha256Digest,
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
            || self.document_state_id.trim().is_empty()
            || self.candidate_ids.is_empty()
            || self.document_family_id.trim().is_empty()
            || !SPLITS.contains(&self.assigned_split.as_str())
            || self.rights_snapshot_ids.is_empty()
        {
            return Err(SupervisionContractError::Missing(
                "training example state, family, split, candidates, or rights",
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
            Some(&self.datasheet_sketch_digest),
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
        unique(self.rights_snapshot_ids.iter(), "training rights snapshot")
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
