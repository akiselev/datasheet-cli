// SPDX-License-Identifier: MIT OR Apache-2.0
//! Durable, provider-independent contracts for evidence-grounded datasheet extraction.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const EVIDENCE_ANCHOR_SCHEMA: &str = "evidence-anchor/2";
pub const CLAIM_BUNDLE_SCHEMA: &str = "claim-bundle/1";
pub const DESIGN_CONSTRAINT_BUNDLE_SCHEMA: &str = "design-constraint-bundle/1";

/// A normalized SHA-256 digest. Constructors in compiler code must validate its source.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct Sha256Digest(pub String);

impl Sha256Digest {
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut encoded = String::with_capacity(64);
        for byte in digest {
            write!(encoded, "{byte:02x}").expect("writing to a string cannot fail");
        }
        Self(encoded)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.0.len() == 64
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            Ok(())
        } else {
            Err(ContractError::InvalidDigest(self.0.clone()))
        }
    }
}

#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct CandidateId(pub String);

#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SketchId(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GeometryQuality {
    ExactGlyph,
    ExactWord,
    ExactLine,
    ExactRegion,
    PageOnly,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceGeometry {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl SourceGeometry {
    pub fn validate(&self) -> Result<(), ContractError> {
        let coordinates = [self.left, self.top, self.right, self.bottom];
        if coordinates
            .iter()
            .all(|coordinate| coordinate.is_finite() && (0.0..=1.0).contains(coordinate))
            && self.left <= self.right
            && self.top <= self.bottom
        {
            Ok(())
        } else {
            Err(ContractError::InvalidGeometry)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentSelector {
    WholeEvidence,
    TextRange {
        evidence_id: String,
        start: u32,
        end: u32,
    },
    TableCell {
        table_region_id: String,
        row: u32,
        column: u32,
    },
}

/// State-aware reference to exact document evidence. Checkpoint identity is optional because
/// logical state identity, rather than physical materialization, owns replay semantics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAnchor {
    pub schema_version: String,
    pub source_pdf_sha256: Sha256Digest,
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub evidence_ids: Vec<String>,
    pub page_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_id: Option<String>,
    pub source_geometry: Vec<SourceGeometry>,
    pub geometry_quality: GeometryQuality,
    pub content_selector: ContentSelector,
}

impl EvidenceAnchor {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != EVIDENCE_ANCHOR_SCHEMA {
            return Err(ContractError::SchemaVersion(self.schema_version.clone()));
        }
        self.source_pdf_sha256.validate()?;
        if let Some(checkpoint) = &self.document_ir_logical_sha256 {
            checkpoint.validate()?;
        }
        if self.document_state_id.is_empty()
            || self.page_id.is_empty()
            || self.evidence_ids.is_empty()
        {
            return Err(ContractError::MissingField("anchor identity/evidence"));
        }
        let evidence: BTreeSet<_> = self.evidence_ids.iter().collect();
        if evidence.len() != self.evidence_ids.len() {
            return Err(ContractError::DuplicateId("evidence id"));
        }
        for geometry in &self.source_geometry {
            geometry.validate()?;
        }
        match &self.content_selector {
            ContentSelector::WholeEvidence => {}
            ContentSelector::TextRange {
                evidence_id,
                start,
                end,
            } => {
                if !evidence.contains(evidence_id) {
                    return Err(ContractError::MissingReference(evidence_id.clone()));
                }
                if start >= end {
                    return Err(ContractError::InvalidTextRange);
                }
            }
            ContentSelector::TableCell {
                table_region_id, ..
            } => {
                if self.region_id.as_ref() != Some(table_region_id) {
                    return Err(ContractError::MissingReference(table_region_id.clone()));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Part,
    Family,
    PackageVariant,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubjectCandidate {
    pub id: CandidateId,
    pub kind: SubjectKind,
    pub raw_designation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_search_key: Option<String>,
    pub evidence_anchor: EvidenceAnchor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ParsedConditionCandidate {
    pub fields: BTreeMap<String, String>,
}

/// A sketch-owned, anchored applicability or test-condition candidate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConditionCandidate {
    pub id: CandidateId,
    pub raw_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_candidate: Option<ParsedConditionCandidate>,
    pub evidence_anchors: Vec<EvidenceAnchor>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParsedQuantityShape {
    Scalar,
    Range,
    Inequality,
    Tolerance,
    Set,
    Symbolic,
    TextUnparsed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NormalizedQuantity {
    pub canonical_value: String,
    pub canonical_unit: String,
    pub registry_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QuantityCandidate {
    pub id: CandidateId,
    pub raw_text: String,
    pub parsed_shape: ParsedQuantityShape,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_quantity: Option<NormalizedQuantity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity_kind_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensionality: Option<String>,
    pub anchor: EvidenceAnchor,
    pub parse_warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpecificationRegime {
    AbsoluteMaximum,
    RecommendedOperating,
    Guaranteed,
    Tested,
    CharacterizedTypical,
    ApplicationGuidance,
    Informative,
    Derived,
    Inferred,
    Measured,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProducerIdentity {
    pub name: String,
    pub version: String,
    pub code_revision: String,
    pub configuration_digest: Sha256Digest,
    pub document_ir_schema: String,
    pub ontology_snapshot_digest: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantitas_registry_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_digest: Option<Sha256Digest>,
}

impl ProducerIdentity {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.name.is_empty()
            || self.version.is_empty()
            || self.code_revision.is_empty()
            || self.document_ir_schema.is_empty()
        {
            return Err(ContractError::MissingField("producer identity"));
        }
        self.configuration_digest.validate()?;
        self.ontology_snapshot_digest.validate()?;
        if let Some(model) = &self.model_digest {
            model.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubjectHint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_raw: Option<String>,
    pub mpn_raw: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroundingStatus {
    Grounded,
    Rejected,
}

/// Semantic role of a selected value within its source table row.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValueRole {
    Minimum,
    Typical,
    Maximum,
    Rating,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClaimCandidate {
    pub local_id: CandidateId,
    pub subject_candidate_id: CandidateId,
    pub predicate_term_id: String,
    pub regime: SpecificationRegime,
    pub value_role: ValueRole,
    pub value_candidate_id: CandidateId,
    pub condition_candidate_ids: Vec<CandidateId>,
    pub package_variant_candidate_ids: Vec<CandidateId>,
    pub evidence_anchors: Vec<EvidenceAnchor>,
    pub extraction_confidence: f64,
    pub grounding_status: GroundingStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimBundle {
    pub schema_version: String,
    pub sketch_id: SketchId,
    pub source_pdf_sha256: Sha256Digest,
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub producer: ProducerIdentity,
    pub subject_hints: Vec<SubjectHint>,
    /// Exact selected value records preserve original source spelling and normalization.
    pub selected_values: Vec<QuantityCandidate>,
    pub claims: Vec<ClaimCandidate>,
    pub warnings: Vec<String>,
}

impl ClaimBundle {
    pub fn validate_structure(&self) -> Result<(), ContractError> {
        if self.schema_version != CLAIM_BUNDLE_SCHEMA {
            return Err(ContractError::SchemaVersion(self.schema_version.clone()));
        }
        self.source_pdf_sha256.validate()?;
        self.producer.validate()?;
        if self.document_state_id.is_empty() {
            return Err(ContractError::MissingField("document_state_id"));
        }
        let selected: BTreeSet<_> = self.selected_values.iter().map(|value| &value.id).collect();
        if selected.len() != self.selected_values.len() {
            return Err(ContractError::DuplicateId("selected value"));
        }
        let claim_ids: BTreeSet<_> = self.claims.iter().map(|claim| &claim.local_id).collect();
        if claim_ids.len() != self.claims.len() {
            return Err(ContractError::DuplicateId("claim"));
        }
        for claim in &self.claims {
            if claim.grounding_status != GroundingStatus::Grounded {
                return Err(ContractError::UngroundedClaim(claim.local_id.0.clone()));
            }
            if !(0.0..=1.0).contains(&claim.extraction_confidence) {
                return Err(ContractError::InvalidConfidence);
            }
            if !selected.contains(&claim.value_candidate_id) {
                return Err(ContractError::MissingReference(
                    claim.value_candidate_id.0.clone(),
                ));
            }
            for anchor in &claim.evidence_anchors {
                anchor.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DesignConstraint {
    pub local_id: CandidateId,
    pub predicate_term_id: String,
    pub value_candidate_id: CandidateId,
    pub evidence_anchors: Vec<EvidenceAnchor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DesignConstraintBundle {
    pub schema_version: String,
    pub sketch_id: SketchId,
    pub producer: ProducerIdentity,
    pub constraints: Vec<DesignConstraint>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PredicateTerm {
    pub id: String,
    pub canonical_name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub symbol_aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_quantity_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_dimensionality: Option<String>,
    pub expected_shapes: Vec<ParsedQuantityShape>,
    pub regime_constraints: Vec<SpecificationRegime>,
    pub category_scope: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PredicateVocabularySnapshot {
    pub schema_version: String,
    pub digest: Sha256Digest,
    pub terms: Vec<PredicateTerm>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    DuplicateId(&'static str),
    InvalidConfidence,
    InvalidDigest(String),
    InvalidGeometry,
    InvalidTextRange,
    MissingField(&'static str),
    MissingReference(String),
    SchemaVersion(String),
    UngroundedClaim(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContractError {}

#[must_use]
pub fn tagged_content_id(tag: &str, canonical_bytes: &[u8]) -> CandidateId {
    CandidateId(format!(
        "{tag}:{}",
        Sha256Digest::of_bytes(canonical_bytes).0
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_lower_hex() {
        let digest = Sha256Digest::of_bytes(b"stable");
        assert_eq!(digest.0.len(), 64);
        digest.validate().unwrap();
    }

    #[test]
    fn predicate_vocabulary_is_data_driven() {
        let schema_before = serde_json::to_value(schemars::schema_for!(ClaimBundle)).unwrap();
        let _new_term = PredicateTerm {
            id: "electrical.example".into(),
            canonical_name: "Example".into(),
            description: "A runtime vocabulary term".into(),
            aliases: vec![],
            symbol_aliases: vec![],
            expected_quantity_kind: None,
            expected_dimensionality: None,
            expected_shapes: vec![ParsedQuantityShape::Scalar],
            regime_constraints: vec![],
            category_scope: vec![],
        };
        let schema_after = serde_json::to_value(schemars::schema_for!(ClaimBundle)).unwrap();
        assert_eq!(schema_before, schema_after);
    }

    #[test]
    fn core_manifest_excludes_network_pdf_and_model_runtimes() {
        let manifest = include_str!("../Cargo.toml");
        for forbidden in [
            "reqwest",
            "ureq",
            "lopdf",
            "mupdf",
            "rten",
            "onnxruntime",
            "foundry",
        ] {
            assert!(
                !manifest.lines().any(|line| line.starts_with(forbidden)),
                "datasheet-core must not depend on {forbidden}"
            );
        }
    }
}
