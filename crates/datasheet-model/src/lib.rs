// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stable model artifact and candidate-selection contracts.
//!
//! FDX1 defines the boundary only. Loading an inference runtime or production model belongs to
//! FDX5 and must not be implied by this crate's presence.

use datasheet_core::{CandidateId, Sha256Digest, SpecificationRegime};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MODEL_MANIFEST_SCHEMA: &str = "datasheet-model-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    pub schema_version: String,
    pub base_model_digest: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantized_model_digest: Option<Sha256Digest>,
    pub tokenizer_digest: Sha256Digest,
    pub feature_schema_digest: Sha256Digest,
    pub ontology_snapshot_digest: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_digest: Option<Sha256Digest>,
    pub dataset_manifest_digest: Sha256Digest,
    pub code_revision: String,
    pub training_configuration_digest: Sha256Digest,
    pub evaluation_report_digest: Sha256Digest,
    pub license_provenance: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateSelectionOutput {
    pub subject_candidate_id: CandidateId,
    pub predicate_candidate_id: CandidateId,
    pub value_candidate_id: CandidateId,
    pub regime: SpecificationRegime,
    pub condition_candidate_ids: Vec<CandidateId>,
    pub confidence: f64,
}
