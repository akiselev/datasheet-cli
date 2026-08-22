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

impl ModelManifest {
    pub fn validate(&self) -> Result<(), ModelContractError> {
        if self.schema_version != MODEL_MANIFEST_SCHEMA {
            return Err(ModelContractError::SchemaVersion(
                self.schema_version.clone(),
            ));
        }
        for digest in [
            Some(&self.base_model_digest),
            self.quantized_model_digest.as_ref(),
            Some(&self.tokenizer_digest),
            Some(&self.feature_schema_digest),
            Some(&self.ontology_snapshot_digest),
            self.calibration_digest.as_ref(),
            Some(&self.dataset_manifest_digest),
            Some(&self.training_configuration_digest),
            Some(&self.evaluation_report_digest),
        ]
        .into_iter()
        .flatten()
        {
            digest
                .validate()
                .map_err(|_| ModelContractError::InvalidDigest(digest.0.clone()))?;
        }
        if self.code_revision.trim().is_empty() || self.license_provenance.trim().is_empty() {
            return Err(ModelContractError::MissingProvenance);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelContractError {
    InvalidDigest(String),
    MissingProvenance,
    SchemaVersion(String),
}

impl std::fmt::Display for ModelContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ModelContractError {}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> Sha256Digest {
        Sha256Digest(std::iter::repeat_n(byte, 64).collect())
    }

    fn manifest() -> ModelManifest {
        ModelManifest {
            schema_version: MODEL_MANIFEST_SCHEMA.into(),
            base_model_digest: digest('1'),
            quantized_model_digest: Some(digest('2')),
            tokenizer_digest: digest('3'),
            feature_schema_digest: digest('4'),
            ontology_snapshot_digest: digest('5'),
            calibration_digest: Some(digest('6')),
            dataset_manifest_digest: digest('7'),
            code_revision: "model-source-revision".into(),
            training_configuration_digest: digest('8'),
            evaluation_report_digest: digest('9'),
            license_provenance: "synthetic test artifact".into(),
        }
    }

    #[test]
    fn manifest_binds_every_required_artifact_and_refuses_drift() {
        manifest().validate().unwrap();
        let mut invalid = manifest();
        invalid.training_configuration_digest = Sha256Digest("not-a-digest".into());
        assert!(invalid.validate().is_err());
        let mut wrong_schema = manifest();
        wrong_schema.schema_version = "datasheet-model-manifest/99".into();
        assert!(wrong_schema.validate().is_err());
    }

    #[test]
    fn checked_in_model_manifest_schema_is_current() {
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/fdx1/model-manifest-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(schemars::schema_for!(ModelManifest)).unwrap(),
            expected
        );
    }
}
