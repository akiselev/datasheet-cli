// SPDX-License-Identifier: MIT OR Apache-2.0
//! Auditable deterministic weak-supervision compiler for FDX3.

use std::collections::{BTreeMap, BTreeSet};

use datasheet_core::supervision::{
    CALIBRATION_SCHEMA, CalibrationSnapshot, LabelVote, LabelerCalibration, LabelerRef,
    REVIEWED_DEVELOPMENT_SET_SCHEMA, ReviewedDevelopmentExample, ReviewedDevelopmentSet,
    SOURCE_VOCABULARY_MAP_SCHEMA, SourceVocabularyMap, SupervisionClass, SupervisionContractError,
    SupervisionObservation, SupervisionProducer, SupervisionUncertainty, TRAINING_LABEL_SCHEMA,
    TrainingLabel, VoteOutcome,
};
use datasheet_core::{CandidateId, PredicateVocabularySnapshot, Sha256Digest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const COMBINER_AGREEMENT_INDEPENDENT: &str = "agreement.independent.v1";
pub const COMBINER_AGREEMENT_STRUCTURED_DOCUMENT: &str = "agreement.structured_document.v1";
pub const COMBINER_AGREEMENT_SUBJECT_TRIANGULATION: &str = "agreement.subject_triangulation.v1";
pub const COMBINER_CONFLICT_SOURCE: &str = "conflict.source.v1";
pub const LABELER_CATALOG_SCHEMA: &str = "fdx3-labeler-catalog/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelerRole {
    HardGate,
    DeterministicHigh,
    Weak,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LabelerDescriptor {
    pub id: &'static str,
    pub version: &'static str,
    pub role: LabelerRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CatalogLabelerRole {
    HardGate,
    DeterministicHigh,
    Weak,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogLabeler {
    pub labeler_id: String,
    pub labeler_version: String,
    pub role: CatalogLabelerRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LabelerCatalogArtifact {
    pub schema_version: String,
    pub digest: Sha256Digest,
    pub labelers: Vec<CatalogLabeler>,
    pub combiners: Vec<String>,
}

const HIGH: LabelerRole = LabelerRole::DeterministicHigh;
const WEAK: LabelerRole = LabelerRole::Weak;
const GATE: LabelerRole = LabelerRole::HardGate;

/// Complete stable FDX3 labeler catalog required by the cross-repository plan.
pub const LABELER_CATALOG: &[LabelerDescriptor] = &[
    LabelerDescriptor {
        id: "section.heading.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "section.table_header_schema.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "cell.min_typ_max_header.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "quantity.dimension.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "predicate.symbol_alias.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "predicate.row_name_alias.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "alignment.value_unique.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "alignment.range.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "template.cluster.v1",
        version: "1",
        role: WEAK,
    },
    LabelerDescriptor {
        id: "applicability.footnote_link.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "scope.package.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "scope.part.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "value.null_dash_convention.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "ranking.regime_conflict.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "revision.structure_stability.v1",
        version: "1",
        role: WEAK,
    },
    LabelerDescriptor {
        id: "ti.identity_gate.v1",
        version: "1",
        role: GATE,
    },
    LabelerDescriptor {
        id: "ti.field_map.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "ti.value_align_unique.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "ti.range_align.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "ti.unit_kind.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "ti.package_align.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "digikey.parameter_id_map.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "digikey.parameter_text_alias.v1",
        version: "1",
        role: WEAK,
    },
    LabelerDescriptor {
        id: "digikey.value_align_unique.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "digikey.category_prior.v1",
        version: "1",
        role: WEAK,
    },
    LabelerDescriptor {
        id: "digikey.package_scope.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "digikey.datasheet_link.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "mouser.mpn_identity.v1",
        version: "1",
        role: GATE,
    },
    LabelerDescriptor {
        id: "mouser.datasheet_link.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "mouser.category.v1",
        version: "1",
        role: WEAK,
    },
    LabelerDescriptor {
        id: "mouser.package.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "mouser.lifecycle.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "mouser.replacement.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.field_map.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.value_align.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.range_align.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.package.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.variant_scope.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.order_code.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "manufacturer.family_scope.v1",
        version: "1",
        role: WEAK,
    },
    LabelerDescriptor {
        id: "eda.pin_number_set.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.pin_name_by_number.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.pin_count.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.package_code.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.pad_count.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.pin_to_pad.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.body_dimensions.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "eda.model_package.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "cmsis.device_identity.v1",
        version: "1",
        role: GATE,
    },
    LabelerDescriptor {
        id: "cmsis.cpu_core.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "cmsis.memory.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "cmsis.document_link.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "svd.peripheral.v1",
        version: "1",
        role: HIGH,
    },
    LabelerDescriptor {
        id: "svd.register.v1",
        version: "1",
        role: HIGH,
    },
];

#[derive(Clone, Debug, PartialEq)]
pub struct SourceVocabularyMapInput {
    pub source_schema: String,
    pub external_namespace: String,
    pub external_field_key: String,
    pub predicate_term_id: String,
    pub expected_quantity_kind: Option<String>,
    pub expected_value_shapes: Vec<datasheet_core::ParsedQuantityShape>,
    pub regime_hint: Option<String>,
    pub category_scope: Vec<String>,
    pub mapping_observation_ids: Vec<Sha256Digest>,
    pub producer: SupervisionProducer,
    pub mapping_version: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LabelVoteInput {
    pub training_candidate_id: CandidateId,
    pub task: String,
    pub labeler: LabelerRef,
    pub outcome: VoteOutcome,
    pub observations: Vec<SupervisionObservation>,
    pub reason_codes: Vec<String>,
    pub producer: SupervisionProducer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CombinationAbstentionReason {
    HardIdentityGate,
    RightsForbidden,
    NoPositiveVotes,
    SourceConflict,
    MissingCalibration,
    InsufficientIndependentLineage,
    LowPosterior,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CombinationAbstention {
    pub record_id: Sha256Digest,
    pub training_candidate_id: CandidateId,
    pub task: String,
    pub reason: CombinationAbstentionReason,
    pub input_vote_record_ids: Vec<Sha256Digest>,
    pub reason_codes: Vec<String>,
    pub combiner_id: String,
    pub combiner_version: String,
    pub combiner_configuration_digest: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum CombinationResult {
    Label { label: TrainingLabel },
    Abstain { abstention: CombinationAbstention },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupervisionError {
    Contract(SupervisionContractError),
    IdentityMismatch(&'static str),
    Invalid(&'static str),
    UnknownLabeler(String),
    Vocabulary(String),
    Serialization(String),
}

impl std::fmt::Display for SupervisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SupervisionError {}

impl From<SupervisionContractError> for SupervisionError {
    fn from(value: SupervisionContractError) -> Self {
        Self::Contract(value)
    }
}

impl From<serde_json::Error> for SupervisionError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}

#[must_use]
pub fn labeler_descriptor(reference: &LabelerRef) -> Option<&'static LabelerDescriptor> {
    LABELER_CATALOG.iter().find(|descriptor| {
        descriptor.id == reference.labeler_id && descriptor.version == reference.labeler_version
    })
}

pub fn frozen_labeler_catalog() -> Result<LabelerCatalogArtifact, SupervisionError> {
    let mut labelers = LABELER_CATALOG
        .iter()
        .map(|descriptor| CatalogLabeler {
            labeler_id: descriptor.id.into(),
            labeler_version: descriptor.version.into(),
            role: match descriptor.role {
                LabelerRole::HardGate => CatalogLabelerRole::HardGate,
                LabelerRole::DeterministicHigh => CatalogLabelerRole::DeterministicHigh,
                LabelerRole::Weak => CatalogLabelerRole::Weak,
            },
        })
        .collect::<Vec<_>>();
    labelers.sort_by(|left, right| {
        (&left.labeler_id, &left.labeler_version).cmp(&(&right.labeler_id, &right.labeler_version))
    });
    let combiners = vec![
        COMBINER_AGREEMENT_INDEPENDENT.into(),
        COMBINER_AGREEMENT_STRUCTURED_DOCUMENT.into(),
        COMBINER_AGREEMENT_SUBJECT_TRIANGULATION.into(),
        COMBINER_CONFLICT_SOURCE.into(),
    ];
    let mut artifact = LabelerCatalogArtifact {
        schema_version: LABELER_CATALOG_SCHEMA.into(),
        digest: Sha256Digest::of_bytes(b"placeholder"),
        labelers,
        combiners,
    };
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: &'a str,
        labelers: &'a [CatalogLabeler],
        combiners: &'a [String],
    }
    artifact.digest = hash(&View {
        schema_version: &artifact.schema_version,
        labelers: &artifact.labelers,
        combiners: &artifact.combiners,
    })?;
    Ok(artifact)
}

pub fn derive_source_vocabulary_map(
    mut input: SourceVocabularyMapInput,
    vocabulary: &PredicateVocabularySnapshot,
) -> Result<SourceVocabularyMap, SupervisionError> {
    vocabulary
        .digest
        .validate()
        .map_err(|_| SupervisionError::Vocabulary("invalid ontology digest".into()))?;
    if !vocabulary
        .terms
        .iter()
        .any(|term| term.id == input.predicate_term_id)
    {
        return Err(SupervisionError::Vocabulary(input.predicate_term_id));
    }
    if input.source_schema.trim().is_empty()
        || input.external_namespace.trim().is_empty()
        || input.external_field_key.trim().is_empty()
        || input.mapping_version.trim().is_empty()
        || input.mapping_observation_ids.is_empty()
    {
        return Err(SupervisionError::Invalid("incomplete vocabulary mapping"));
    }
    input.producer.validate()?;
    sort_unique(&mut input.mapping_observation_ids, "mapping observation")?;
    input.expected_value_shapes.sort();
    input.expected_value_shapes.dedup();
    input.category_scope.sort();
    input.category_scope.dedup();
    let mut mapping = SourceVocabularyMap {
        schema_version: SOURCE_VOCABULARY_MAP_SCHEMA.into(),
        map_id: Sha256Digest::of_bytes(b"placeholder"),
        source_schema: input.source_schema,
        external_namespace: input.external_namespace,
        external_field_key: input.external_field_key,
        predicate_term_id: input.predicate_term_id,
        expected_quantity_kind: input.expected_quantity_kind,
        expected_value_shapes: input.expected_value_shapes,
        regime_hint: input.regime_hint,
        category_scope: input.category_scope,
        mapping_observation_ids: input.mapping_observation_ids,
        ontology_snapshot_digest: vocabulary.digest.clone(),
        producer: input.producer,
        mapping_version: input.mapping_version,
    };
    mapping.map_id = source_vocabulary_map_id(&mapping)?;
    validate_source_vocabulary_map(&mapping, vocabulary)?;
    Ok(mapping)
}

pub fn source_vocabulary_map_id(
    mapping: &SourceVocabularyMap,
) -> Result<Sha256Digest, SupervisionError> {
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: &'a str,
        source_schema: &'a str,
        external_namespace: &'a str,
        external_field_key: &'a str,
        predicate_term_id: &'a str,
        expected_quantity_kind: &'a Option<String>,
        expected_value_shapes: &'a [datasheet_core::ParsedQuantityShape],
        regime_hint: &'a Option<String>,
        category_scope: &'a [String],
        mapping_observation_ids: &'a [Sha256Digest],
        ontology_snapshot_digest: &'a Sha256Digest,
        producer: &'a SupervisionProducer,
        mapping_version: &'a str,
    }
    hash(&View {
        schema_version: &mapping.schema_version,
        source_schema: &mapping.source_schema,
        external_namespace: &mapping.external_namespace,
        external_field_key: &mapping.external_field_key,
        predicate_term_id: &mapping.predicate_term_id,
        expected_quantity_kind: &mapping.expected_quantity_kind,
        expected_value_shapes: &mapping.expected_value_shapes,
        regime_hint: &mapping.regime_hint,
        category_scope: &mapping.category_scope,
        mapping_observation_ids: &mapping.mapping_observation_ids,
        ontology_snapshot_digest: &mapping.ontology_snapshot_digest,
        producer: &mapping.producer,
        mapping_version: &mapping.mapping_version,
    })
}

pub fn validate_source_vocabulary_map(
    mapping: &SourceVocabularyMap,
    vocabulary: &PredicateVocabularySnapshot,
) -> Result<(), SupervisionError> {
    if mapping.schema_version != SOURCE_VOCABULARY_MAP_SCHEMA
        || mapping.ontology_snapshot_digest != vocabulary.digest
        || !vocabulary
            .terms
            .iter()
            .any(|term| term.id == mapping.predicate_term_id)
        || mapping.source_schema.trim().is_empty()
        || mapping.external_namespace.trim().is_empty()
        || mapping.external_field_key.trim().is_empty()
        || mapping.mapping_version.trim().is_empty()
        || mapping.mapping_observation_ids.is_empty()
    {
        return Err(SupervisionError::Invalid("source vocabulary map"));
    }
    mapping.producer.validate()?;
    if !strictly_sorted(&mapping.mapping_observation_ids)
        || !strictly_sorted(&mapping.expected_value_shapes)
        || !strictly_sorted(&mapping.category_scope)
    {
        return Err(SupervisionError::Invalid(
            "noncanonical vocabulary map order",
        ));
    }
    if mapping.map_id != source_vocabulary_map_id(mapping)? {
        return Err(SupervisionError::IdentityMismatch("source vocabulary map"));
    }
    Ok(())
}

pub fn build_label_vote(mut input: LabelVoteInput) -> Result<LabelVote, SupervisionError> {
    if labeler_descriptor(&input.labeler).is_none() {
        return Err(SupervisionError::UnknownLabeler(input.labeler.labeler_id));
    }
    for observation in &mut input.observations {
        observation.candidate_ids.sort();
        reject_adjacent_duplicates(observation.candidate_ids.iter(), "observation candidate")?;
        let mut anchors = observation
            .evidence_anchors
            .drain(..)
            .map(|anchor| Ok((serde_json::to_vec(&anchor)?, anchor)))
            .collect::<Result<Vec<_>, SupervisionError>>()?;
        anchors.sort_by(|left, right| left.0.cmp(&right.0));
        reject_adjacent_duplicates(anchors.iter().map(|(bytes, _)| bytes), "evidence anchor")?;
        observation.evidence_anchors = anchors.into_iter().map(|(_, anchor)| anchor).collect();
        observation
            .lineage_groups
            .sort_by(|left, right| left.group_id.cmp(&right.group_id));
        observation
            .rights_snapshots
            .sort_by(|left, right| left.snapshot_id.cmp(&right.snapshot_id));
    }
    input
        .observations
        .sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    reject_adjacent_duplicates(
        input
            .observations
            .iter()
            .map(|observation| &observation.observation_id),
        "observation",
    )?;
    sort_unique(&mut input.reason_codes, "reason code")?;
    let mut lineage_group_ids = input
        .observations
        .iter()
        .flat_map(|observation| {
            observation
                .lineage_groups
                .iter()
                .map(|group| group.group_id.clone())
        })
        .collect::<Vec<_>>();
    lineage_group_ids.sort();
    lineage_group_ids.dedup();
    let mut vote = LabelVote {
        schema_version: datasheet_core::supervision::LABEL_VOTE_SCHEMA.into(),
        vote_id: Sha256Digest::of_bytes(b"placeholder"),
        training_candidate_id: input.training_candidate_id,
        task: input.task,
        labeler: input.labeler,
        outcome: input.outcome,
        observations: input.observations,
        lineage_group_ids,
        reason_codes: input.reason_codes,
        producer: input.producer,
    };
    vote.vote_id = label_vote_id(&vote)?;
    vote.validate_structure()?;
    Ok(vote)
}

pub fn validate_label_vote(vote: &LabelVote) -> Result<(), SupervisionError> {
    vote.validate_structure()?;
    if labeler_descriptor(&vote.labeler).is_none() {
        return Err(SupervisionError::UnknownLabeler(
            vote.labeler.labeler_id.clone(),
        ));
    }
    let canonical = build_label_vote(LabelVoteInput {
        training_candidate_id: vote.training_candidate_id.clone(),
        task: vote.task.clone(),
        labeler: vote.labeler.clone(),
        outcome: vote.outcome.clone(),
        observations: vote.observations.clone(),
        reason_codes: vote.reason_codes.clone(),
        producer: vote.producer.clone(),
    })?;
    if &canonical != vote {
        return Err(SupervisionError::IdentityMismatch(
            "label vote or canonical ordering",
        ));
    }
    Ok(())
}

pub fn label_vote_id(vote: &LabelVote) -> Result<Sha256Digest, SupervisionError> {
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: &'a str,
        training_candidate_id: &'a CandidateId,
        task: &'a str,
        labeler: &'a LabelerRef,
        outcome: &'a VoteOutcome,
        observations: &'a [SupervisionObservation],
        lineage_group_ids: &'a [Sha256Digest],
        reason_codes: &'a [String],
        producer: &'a SupervisionProducer,
    }
    hash(&View {
        schema_version: &vote.schema_version,
        training_candidate_id: &vote.training_candidate_id,
        task: &vote.task,
        labeler: &vote.labeler,
        outcome: &vote.outcome,
        observations: &vote.observations,
        lineage_group_ids: &vote.lineage_group_ids,
        reason_codes: &vote.reason_codes,
        producer: &vote.producer,
    })
}

pub fn combine_votes(
    training_candidate_id: &CandidateId,
    task: &str,
    votes: &[LabelVote],
    calibration: &CalibrationSnapshot,
) -> Result<CombinationResult, SupervisionError> {
    calibration.validate_structure()?;
    if !calibration.entries.windows(2).all(|pair| {
        (
            &pair[0].labeler.labeler_id,
            &pair[0].labeler.labeler_version,
        ) < (
            &pair[1].labeler.labeler_id,
            &pair[1].labeler.labeler_version,
        )
    }) {
        return Err(SupervisionError::Invalid(
            "noncanonical calibration ordering",
        ));
    }
    if calibration.digest != calibration_snapshot_digest(calibration)? {
        return Err(SupervisionError::IdentityMismatch("calibration snapshot"));
    }
    let mut ordered = votes.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.vote_id.cmp(&right.vote_id));
    reject_adjacent_duplicates(ordered.iter().map(|vote| &vote.vote_id), "vote record")?;
    for vote in &ordered {
        validate_label_vote(vote)?;
        if &vote.training_candidate_id != training_candidate_id || vote.task != task {
            return Err(SupervisionError::Invalid("mixed candidate or task"));
        }
    }
    let input_ids = ordered
        .iter()
        .map(|vote| vote.vote_id.clone())
        .collect::<Vec<_>>();
    let calibration_by_labeler = calibration
        .entries
        .iter()
        .map(|entry| {
            (
                (&entry.labeler.labeler_id, &entry.labeler.labeler_version),
                entry,
            )
        })
        .collect::<BTreeMap<_, _>>();

    if ordered.iter().any(|vote| {
        matches!(vote.outcome, VoteOutcome::Vote { .. })
            && vote
                .observations
                .iter()
                .any(|observation| !observation.permits_training())
    }) {
        return abstain(
            training_candidate_id,
            task,
            &calibration.digest,
            CombinationAbstentionReason::RightsForbidden,
            input_ids,
            vec!["training_rights_fail_closed".into()],
        );
    }

    for vote in &ordered {
        let descriptor = labeler_descriptor(&vote.labeler).expect("validated labeler");
        if descriptor.role == LabelerRole::HardGate {
            let passed = matches!(&vote.outcome, VoteOutcome::Vote { vote_json } if vote_json == &Value::Bool(true));
            if !passed {
                return abstain(
                    training_candidate_id,
                    task,
                    &calibration.digest,
                    CombinationAbstentionReason::HardIdentityGate,
                    input_ids,
                    vec!["hard_identity_gate_not_satisfied".into()],
                );
            }
        }
    }

    let positive = ordered
        .iter()
        .filter(|vote| {
            labeler_descriptor(&vote.labeler)
                .expect("validated labeler")
                .role
                != LabelerRole::HardGate
                && matches!(vote.outcome, VoteOutcome::Vote { .. })
        })
        .copied()
        .collect::<Vec<_>>();
    if positive.is_empty() {
        return abstain(
            training_candidate_id,
            task,
            &calibration.digest,
            CombinationAbstentionReason::NoPositiveVotes,
            input_ids,
            vec!["no_votes_is_not_negative".into()],
        );
    }
    if positive.iter().any(|vote| {
        !calibration_by_labeler
            .contains_key(&(&vote.labeler.labeler_id, &vote.labeler.labeler_version))
    }) {
        return abstain(
            training_candidate_id,
            task,
            &calibration.digest,
            CombinationAbstentionReason::MissingCalibration,
            input_ids,
            vec!["labeler_not_calibrated".into()],
        );
    }

    let components = lineage_components(&positive);
    let mut scores = BTreeMap::<Vec<u8>, (Value, u128, u32)>::new();
    let mut deterministic_targets = BTreeSet::<Vec<u8>>::new();
    for component in &components {
        let mut targets = BTreeMap::<Vec<u8>, (Value, u32, bool)>::new();
        for index in component {
            let vote = positive[*index];
            let VoteOutcome::Vote { vote_json } = &vote.outcome else {
                unreachable!("positive votes are values")
            };
            let canonical = serde_json::to_vec(vote_json)?;
            let calibration_entry =
                calibration_by_labeler[&(&vote.labeler.labeler_id, &vote.labeler.labeler_version)];
            let deterministic = labeler_descriptor(&vote.labeler)
                .expect("validated labeler")
                .role
                == LabelerRole::DeterministicHigh;
            let entry = targets
                .entry(canonical.clone())
                .or_insert((vote_json.clone(), 0, false));
            entry.1 = entry.1.max(calibration_entry.weight_millionths);
            entry.2 |= deterministic;
            if deterministic {
                deterministic_targets.insert(canonical);
            }
        }
        if targets.len() > 1 {
            return abstain(
                training_candidate_id,
                task,
                &calibration.digest,
                CombinationAbstentionReason::SourceConflict,
                input_ids,
                vec!["conflict_within_shared_lineage".into()],
            );
        }
        let (canonical, (target, weight, _)) =
            targets.into_iter().next().expect("component has vote");
        let score = scores.entry(canonical).or_insert((target, 0, 0));
        score.1 = score
            .1
            .checked_add(u128::from(weight))
            .ok_or(SupervisionError::Invalid("combiner weight overflow"))?;
        score.2 = score
            .2
            .checked_add(1)
            .ok_or(SupervisionError::Invalid("lineage count overflow"))?;
    }
    if deterministic_targets.len() > 1 {
        return abstain(
            training_candidate_id,
            task,
            &calibration.digest,
            CombinationAbstentionReason::SourceConflict,
            input_ids,
            vec!["deterministic_labeler_conflict".into()],
        );
    }
    let chosen_key = if let Some(key) = deterministic_targets.first() {
        key.clone()
    } else {
        let max = scores
            .values()
            .map(|(_, score, _)| *score)
            .max()
            .expect("scores");
        let winners = scores
            .iter()
            .filter(|(_, (_, score, _))| *score == max)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if winners.len() != 1 {
            return abstain(
                training_candidate_id,
                task,
                &calibration.digest,
                CombinationAbstentionReason::SourceConflict,
                input_ids,
                vec!["source_conflict_is_not_negative".into()],
            );
        }
        winners[0].clone()
    };
    let (target, chosen_score, chosen_lineages) = scores
        .get(&chosen_key)
        .cloned()
        .expect("deterministic target was scored");
    if chosen_lineages < calibration.minimum_independent_lineages {
        return abstain(
            training_candidate_id,
            task,
            &calibration.digest,
            CombinationAbstentionReason::InsufficientIndependentLineage,
            input_ids,
            vec!["lineage_deduplicated_support_below_threshold".into()],
        );
    }
    let total_score = scores.values().map(|(_, score, _)| *score).sum::<u128>();
    let posterior = chosen_score
        .checked_mul(1_000_000)
        .ok_or(SupervisionError::Invalid("posterior overflow"))?
        / total_score;
    let posterior = u32::try_from(posterior)
        .map_err(|_| SupervisionError::Invalid("posterior out of range"))?;
    if posterior < calibration.minimum_posterior_millionths {
        return abstain(
            training_candidate_id,
            task,
            &calibration.digest,
            CombinationAbstentionReason::LowPosterior,
            input_ids,
            vec!["low_confidence_is_not_negative".into()],
        );
    }
    let supervision_class = if deterministic_targets.is_empty() {
        SupervisionClass::WeakConsensus
    } else {
        SupervisionClass::DeterministicHigh
    };
    let mut label = TrainingLabel {
        schema_version: TRAINING_LABEL_SCHEMA.into(),
        training_label_id: Sha256Digest::of_bytes(b"placeholder"),
        training_candidate_id: training_candidate_id.clone(),
        task: task.into(),
        target_json: target,
        supervision_class,
        uncertainty: SupervisionUncertainty {
            posterior_millionths: posterior,
            disagreement_millionths: 1_000_000 - posterior,
            independent_lineage_count: chosen_lineages,
            competing_target_count: u32::try_from(scores.len())
                .map_err(|_| SupervisionError::Invalid("too many targets"))?,
        },
        input_vote_record_ids: input_ids,
        combiner_id: COMBINER_AGREEMENT_INDEPENDENT.into(),
        combiner_version: "1".into(),
        combiner_configuration_digest: calibration.digest.clone(),
        review_record_id: None,
    };
    label.training_label_id = training_label_id(&label)?;
    validate_training_label(&label)?;
    Ok(CombinationResult::Label { label })
}

pub fn calibration_snapshot_digest(
    calibration: &CalibrationSnapshot,
) -> Result<Sha256Digest, SupervisionError> {
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: &'a str,
        reviewed_development_set_digest: &'a Sha256Digest,
        ontology_snapshot_digest: &'a Sha256Digest,
        entries: &'a [LabelerCalibration],
        minimum_posterior_millionths: u32,
        minimum_independent_lineages: u32,
        producer: &'a SupervisionProducer,
    }
    hash(&View {
        schema_version: &calibration.schema_version,
        reviewed_development_set_digest: &calibration.reviewed_development_set_digest,
        ontology_snapshot_digest: &calibration.ontology_snapshot_digest,
        entries: &calibration.entries,
        minimum_posterior_millionths: calibration.minimum_posterior_millionths,
        minimum_independent_lineages: calibration.minimum_independent_lineages,
        producer: &calibration.producer,
    })
}

pub fn freeze_calibration_snapshot(
    reviewed_development_set_digest: Sha256Digest,
    ontology_snapshot_digest: Sha256Digest,
    mut entries: Vec<LabelerCalibration>,
    minimum_posterior_millionths: u32,
    minimum_independent_lineages: u32,
    producer: SupervisionProducer,
) -> Result<CalibrationSnapshot, SupervisionError> {
    entries.sort_by(|left, right| {
        (&left.labeler.labeler_id, &left.labeler.labeler_version)
            .cmp(&(&right.labeler.labeler_id, &right.labeler.labeler_version))
    });
    let mut snapshot = CalibrationSnapshot {
        schema_version: CALIBRATION_SCHEMA.into(),
        digest: Sha256Digest::of_bytes(b"placeholder"),
        reviewed_development_set_digest,
        ontology_snapshot_digest,
        entries,
        minimum_posterior_millionths,
        minimum_independent_lineages,
        producer,
    };
    snapshot.digest = calibration_snapshot_digest(&snapshot)?;
    snapshot.validate_structure()?;
    Ok(snapshot)
}

pub fn freeze_reviewed_development_set(
    ontology_snapshot_digest: Sha256Digest,
    mut examples: Vec<ReviewedDevelopmentExample>,
    producer: SupervisionProducer,
) -> Result<ReviewedDevelopmentSet, SupervisionError> {
    if examples.is_empty() {
        return Err(SupervisionError::Invalid("empty reviewed development set"));
    }
    examples.sort_by(|left, right| left.example_id.cmp(&right.example_id));
    reject_adjacent_duplicates(
        examples.iter().map(|example| &example.example_id),
        "development example",
    )?;
    for example in &mut examples {
        if example.task.trim().is_empty()
            || example.expected_target_json.is_null()
            || example.observation_ids.is_empty()
        {
            return Err(SupervisionError::Invalid(
                "incomplete reviewed development example",
            ));
        }
        example
            .example_id
            .validate()
            .map_err(|_| SupervisionError::Invalid("development example ID"))?;
        example
            .reviewer_record_id
            .validate()
            .map_err(|_| SupervisionError::Invalid("review record ID"))?;
        sort_unique(&mut example.observation_ids, "development observation")?;
    }
    ontology_snapshot_digest
        .validate()
        .map_err(|_| SupervisionError::Invalid("ontology digest"))?;
    producer.validate()?;
    let mut development = ReviewedDevelopmentSet {
        schema_version: REVIEWED_DEVELOPMENT_SET_SCHEMA.into(),
        digest: Sha256Digest::of_bytes(b"placeholder"),
        ontology_snapshot_digest,
        examples,
        producer,
    };
    development.digest = reviewed_development_set_digest(&development)?;
    Ok(development)
}

pub fn reviewed_development_set_digest(
    development: &ReviewedDevelopmentSet,
) -> Result<Sha256Digest, SupervisionError> {
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: &'a str,
        ontology_snapshot_digest: &'a Sha256Digest,
        examples: &'a [ReviewedDevelopmentExample],
        producer: &'a SupervisionProducer,
    }
    hash(&View {
        schema_version: &development.schema_version,
        ontology_snapshot_digest: &development.ontology_snapshot_digest,
        examples: &development.examples,
        producer: &development.producer,
    })
}

pub fn training_label_id(label: &TrainingLabel) -> Result<Sha256Digest, SupervisionError> {
    #[derive(Serialize)]
    struct View<'a> {
        schema_version: &'a str,
        training_candidate_id: &'a CandidateId,
        task: &'a str,
        target_json: &'a Value,
        supervision_class: SupervisionClass,
        uncertainty: &'a SupervisionUncertainty,
        input_vote_record_ids: &'a [Sha256Digest],
        combiner_id: &'a str,
        combiner_version: &'a str,
        combiner_configuration_digest: &'a Sha256Digest,
        review_record_id: &'a Option<Sha256Digest>,
    }
    hash(&View {
        schema_version: &label.schema_version,
        training_candidate_id: &label.training_candidate_id,
        task: &label.task,
        target_json: &label.target_json,
        supervision_class: label.supervision_class,
        uncertainty: &label.uncertainty,
        input_vote_record_ids: &label.input_vote_record_ids,
        combiner_id: &label.combiner_id,
        combiner_version: &label.combiner_version,
        combiner_configuration_digest: &label.combiner_configuration_digest,
        review_record_id: &label.review_record_id,
    })
}

pub fn validate_training_label(label: &TrainingLabel) -> Result<(), SupervisionError> {
    if label.schema_version != TRAINING_LABEL_SCHEMA
        || label.training_candidate_id.0.trim().is_empty()
        || label.task.trim().is_empty()
        || label.target_json.is_null()
        || label.input_vote_record_ids.is_empty()
        || !strictly_sorted(&label.input_vote_record_ids)
        || label.combiner_id.trim().is_empty()
        || label.combiner_version.trim().is_empty()
        || label.uncertainty.posterior_millionths > 1_000_000
        || label.uncertainty.disagreement_millionths
            != 1_000_000 - label.uncertainty.posterior_millionths
        || label.uncertainty.independent_lineage_count == 0
        || label.uncertainty.competing_target_count == 0
    {
        return Err(SupervisionError::Invalid("training label"));
    }
    label
        .combiner_configuration_digest
        .validate()
        .map_err(|_| SupervisionError::Invalid("combiner configuration digest"))?;
    if let Some(review) = &label.review_record_id {
        review
            .validate()
            .map_err(|_| SupervisionError::Invalid("review record digest"))?;
    }
    if label.training_label_id != training_label_id(label)? {
        return Err(SupervisionError::IdentityMismatch("training label"));
    }
    Ok(())
}

fn lineage_components(votes: &[&LabelVote]) -> Vec<Vec<usize>> {
    let mut parents = (0..votes.len()).collect::<Vec<_>>();
    for left in 0..votes.len() {
        let left_groups = votes[left]
            .lineage_group_ids
            .iter()
            .collect::<BTreeSet<_>>();
        for (right, vote) in votes.iter().enumerate().skip(left + 1) {
            if vote
                .lineage_group_ids
                .iter()
                .any(|group| left_groups.contains(group))
            {
                union(&mut parents, left, right);
            }
        }
    }
    let mut result = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..votes.len() {
        let root = find(&mut parents, index);
        result.entry(root).or_default().push(index);
    }
    result.into_values().collect()
}

fn find(parents: &mut [usize], index: usize) -> usize {
    if parents[index] != index {
        parents[index] = find(parents, parents[index]);
    }
    parents[index]
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left = find(parents, left);
    let right = find(parents, right);
    let (low, high) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    parents[high] = low;
}

fn abstain(
    candidate: &CandidateId,
    task: &str,
    combiner_configuration_digest: &Sha256Digest,
    reason: CombinationAbstentionReason,
    input_vote_record_ids: Vec<Sha256Digest>,
    reason_codes: Vec<String>,
) -> Result<CombinationResult, SupervisionError> {
    #[derive(Serialize)]
    struct View<'a> {
        training_candidate_id: &'a CandidateId,
        task: &'a str,
        reason: &'a CombinationAbstentionReason,
        input_vote_record_ids: &'a [Sha256Digest],
        reason_codes: &'a [String],
        combiner_id: &'a str,
        combiner_version: &'a str,
        combiner_configuration_digest: &'a Sha256Digest,
    }
    let combiner_id = if reason == CombinationAbstentionReason::SourceConflict {
        COMBINER_CONFLICT_SOURCE
    } else {
        COMBINER_AGREEMENT_INDEPENDENT
    };
    let view = View {
        training_candidate_id: candidate,
        task,
        reason: &reason,
        input_vote_record_ids: &input_vote_record_ids,
        reason_codes: &reason_codes,
        combiner_id,
        combiner_version: "1",
        combiner_configuration_digest,
    };
    Ok(CombinationResult::Abstain {
        abstention: CombinationAbstention {
            record_id: hash(&view)?,
            training_candidate_id: candidate.clone(),
            task: task.into(),
            reason,
            input_vote_record_ids,
            reason_codes,
            combiner_id: combiner_id.into(),
            combiner_version: "1".into(),
            combiner_configuration_digest: combiner_configuration_digest.clone(),
        },
    })
}

fn hash(value: &impl Serialize) -> Result<Sha256Digest, SupervisionError> {
    Ok(Sha256Digest::of_bytes(&serde_json::to_vec(value)?))
}

fn sort_unique<T: Ord>(values: &mut [T], what: &'static str) -> Result<(), SupervisionError> {
    values.sort();
    reject_adjacent_duplicates(values.iter(), what)
}

fn reject_adjacent_duplicates<T: PartialEq>(
    values: impl IntoIterator<Item = T>,
    what: &'static str,
) -> Result<(), SupervisionError> {
    let mut previous = None;
    for value in values {
        if previous.as_ref() == Some(&value) {
            return Err(SupervisionError::Invalid(what));
        }
        previous = Some(value);
    }
    Ok(())
}

fn strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}
