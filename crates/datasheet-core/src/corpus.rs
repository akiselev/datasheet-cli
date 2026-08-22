// SPDX-License-Identifier: MIT OR Apache-2.0
//! Immutable FDX4 protected-corpus and split contracts.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Sha256Digest;

pub const PROTECTED_CORPUS_SCHEMA: &str = "fdx4.protected-corpus-manifest/v1";
pub const PROTECTED_CORPUS_V2_SCHEMA: &str = "fdx4.protected-corpus-manifest/v2";
pub const SEALED_HIDDEN_HANDOFF_SCHEMA: &str = "fdx4.sealed-hidden-evaluation-handoff/v1";

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectedSplit {
    Train,
    Development,
    TemplateHeldOut,
    DocumentFamilyHeldOut,
    ManufacturerHeldOut,
    FinalBlindTest,
}

/// The evidence class controls which claims a protected slice may support.
#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CorpusEvidenceClass {
    RealDocument,
    IndependentlySynthetic,
    SyntheticDerivative,
}

/// A grouping dimension that an evaluation slice promises is disjoint from train.
#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GroupingLevel {
    ExactContent,
    NearDuplicate,
    Revision,
    SyntheticRoot,
    DocumentFamily,
    TemplateFamily,
    Manufacturer,
    PublisherLineage,
}

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SealedHandoffState {
    AwaitingExternalSeal,
    SealedReady,
}

#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SealedArtifactRole {
    SourceBytes,
    LicenseTermsBytes,
    SourcePolicySnapshot,
    AuthorityPolicySnapshot,
    AcquisitionReceipt,
    GroupingManifest,
    ReviewedLabels,
    EvaluationManifest,
}

impl SealedArtifactRole {
    pub const ALL: [Self; 8] = [
        Self::SourceBytes,
        Self::LicenseTermsBytes,
        Self::SourcePolicySnapshot,
        Self::AuthorityPolicySnapshot,
        Self::AcquisitionReceipt,
        Self::GroupingManifest,
        Self::ReviewedLabels,
        Self::EvaluationManifest,
    ];
}

/// Repository-safe contract for an independently operated final-blind evaluation.
///
/// Source bytes, labels, and decryption credentials must remain outside the shared
/// repository and outside the candidate process. The checked-in form may request a seal,
/// but cannot represent a completed hidden evaluation without an external attestation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SealedHiddenEvaluationHandoff {
    pub schema_version: String,
    pub handoff_digest: Sha256Digest,
    pub qualification_context_digest: Sha256Digest,
    pub training_group_commitment_sha256: Sha256Digest,
    pub split: ProtectedSplit,
    pub state: SealedHandoffState,
    pub external_store_locator: String,
    pub held_out_from_train_by: Vec<GroupingLevel>,
    pub required_evidence_class: CorpusEvidenceClass,
    pub required_artifact_roles: Vec<SealedArtifactRole>,
    pub retention_permission_required: bool,
    pub training_permission_required: bool,
    pub policy_fail_closed: bool,
    pub source_bytes_repository_visible: bool,
    pub labels_repository_visible: bool,
    pub candidate_network_access: bool,
    pub candidate_credentials_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sealed_manifest_content_sha256: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifactum_collection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_attestation_sha256: Option<Sha256Digest>,
    pub limitation_codes: Vec<String>,
}

impl SealedHiddenEvaluationHandoff {
    #[must_use]
    pub fn recompute_digest(&self) -> Sha256Digest {
        let mut canonical = self.clone();
        canonical.handoff_digest = Sha256Digest::of_bytes(b"placeholder");
        canonical.held_out_from_train_by.sort();
        canonical.required_artifact_roles.sort();
        canonical.limitation_codes.sort();
        Sha256Digest::of_bytes(&serde_json::to_vec(&canonical).expect("handoff serialization"))
    }

    pub fn validate(&self) -> Result<(), CorpusContractError> {
        if self.schema_version != SEALED_HIDDEN_HANDOFF_SCHEMA
            || self.split != ProtectedSplit::FinalBlindTest
            || !self
                .external_store_locator
                .starts_with("artifactum+sealed://")
            || self.held_out_from_train_by.is_empty()
            || self.required_artifact_roles.len() != SealedArtifactRole::ALL.len()
            || self.limitation_codes.is_empty()
        {
            return Err(CorpusContractError::Invalid("sealed handoff identity"));
        }
        if self.source_bytes_repository_visible
            || self.labels_repository_visible
            || self.candidate_network_access
            || self.candidate_credentials_available
            || self.required_evidence_class != CorpusEvidenceClass::RealDocument
            || !self.retention_permission_required
            || !self.training_permission_required
            || !self.policy_fail_closed
        {
            return Err(CorpusContractError::Invalid("hidden evaluation isolation"));
        }
        self.qualification_context_digest
            .validate()
            .map_err(|_| CorpusContractError::Invalid("handoff context digest"))?;
        self.training_group_commitment_sha256
            .validate()
            .map_err(|_| CorpusContractError::Invalid("training grouping commitment"))?;
        unique(self.held_out_from_train_by.iter(), "handoff grouping")?;
        unique(self.required_artifact_roles.iter(), "handoff artifact role")?;
        if !SealedArtifactRole::ALL
            .iter()
            .all(|role| self.required_artifact_roles.contains(role))
        {
            return Err(CorpusContractError::Missing(
                "sealed rights/provenance artifact",
            ));
        }
        unique(self.limitation_codes.iter(), "handoff limitation")?;
        let sealed = self.sealed_manifest_content_sha256.is_some()
            && self.artifactum_collection_id.is_some()
            && self.operator_attestation_sha256.is_some();
        if (self.state == SealedHandoffState::SealedReady) != sealed {
            return Err(CorpusContractError::Invalid("sealed handoff readiness"));
        }
        if let Some(digest) = &self.sealed_manifest_content_sha256 {
            digest
                .validate()
                .map_err(|_| CorpusContractError::Invalid("sealed manifest digest"))?;
        }
        if let Some(id) = &self.artifactum_collection_id {
            let digest = id
                .strip_prefix("sha256:")
                .ok_or(CorpusContractError::Invalid("sealed Artifactum collection"))?;
            Sha256Digest(digest.into())
                .validate()
                .map_err(|_| CorpusContractError::Invalid("sealed Artifactum collection"))?;
        }
        if let Some(digest) = &self.operator_attestation_sha256 {
            digest
                .validate()
                .map_err(|_| CorpusContractError::Invalid("operator attestation"))?;
        }
        if self.handoff_digest != self.recompute_digest() {
            return Err(CorpusContractError::Invalid("sealed handoff digest"));
        }
        Ok(())
    }

    #[must_use]
    pub fn ready_for_hidden_evaluation(&self) -> bool {
        self.state == SealedHandoffState::SealedReady && self.validate().is_ok()
    }
}

impl ProtectedSplit {
    pub const ALL: [Self; 6] = [
        Self::Train,
        Self::Development,
        Self::TemplateHeldOut,
        Self::DocumentFamilyHeldOut,
        Self::ManufacturerHeldOut,
        Self::FinalBlindTest,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusInputBinding {
    pub role: String,
    pub content_sha256: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_id: Option<String>,
    pub row_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusMember {
    pub example_id: Sha256Digest,
    pub example_content_sha256: Sha256Digest,
    pub assigned_split: ProtectedSplit,
    pub document_family_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_family_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_key: Option<String>,
    pub lineage_group_ids: Vec<Sha256Digest>,
    pub near_duplicate_group_id: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_group_id: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic_root_example_id: Option<Sha256Digest>,
    pub rights_snapshot_ids: Vec<Sha256Digest>,
    pub training_permitted: bool,
    pub restriction_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusExclusion {
    pub example_id: Sha256Digest,
    pub reason_codes: Vec<String>,
    pub rights_snapshot_ids: Vec<Sha256Digest>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectedSplitManifest {
    pub split: ProtectedSplit,
    pub member_ids: Vec<Sha256Digest>,
    pub hidden: bool,
    pub artifact_digest: Sha256Digest,
    pub limitation_codes: Vec<String>,
}

impl ProtectedSplitManifest {
    #[must_use]
    pub fn recompute_digest(&self) -> Sha256Digest {
        let mut canonical = self.clone();
        canonical.member_ids.sort();
        canonical.limitation_codes.sort();
        canonical.artifact_digest = Sha256Digest::of_bytes(b"placeholder");
        Sha256Digest::of_bytes(&serde_json::to_vec(&canonical).expect("split serialization"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectedCorpusManifest {
    pub schema_version: String,
    pub corpus_digest: Sha256Digest,
    pub datasheet_revision: String,
    pub foundry_revision: String,
    pub ontology_snapshot_digest: Sha256Digest,
    pub calibration_snapshot_digest: Sha256Digest,
    pub builder_configuration_digest: Sha256Digest,
    pub inputs: Vec<CorpusInputBinding>,
    pub members: Vec<CorpusMember>,
    pub exclusions: Vec<CorpusExclusion>,
    pub splits: Vec<ProtectedSplitManifest>,
    pub model_qualification_ready: bool,
    pub limitation_codes: Vec<String>,
}

/// A corpus member whose membership is projected by one or more slice tags.
///
/// Unlike v1, evaluation tags may overlap. They are not partitions: every evaluation
/// slice independently proves its declared grouping dimensions disjoint from train.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorpusMemberV2 {
    pub example_id: Sha256Digest,
    pub example_content_sha256: Sha256Digest,
    pub evidence_class: CorpusEvidenceClass,
    pub real_world_generalization_eligible: bool,
    pub document_family_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_family_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_key: Option<String>,
    pub lineage_group_ids: Vec<Sha256Digest>,
    pub near_duplicate_group_id: Sha256Digest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_group_id: Option<Sha256Digest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic_root_example_id: Option<Sha256Digest>,
    pub rights_snapshot_ids: Vec<Sha256Digest>,
    pub training_permitted: bool,
    pub restriction_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectedSliceManifestV2 {
    pub split: ProtectedSplit,
    pub member_ids: Vec<Sha256Digest>,
    pub held_out_from_train_by: Vec<GroupingLevel>,
    pub hidden: bool,
    pub artifact_digest: Sha256Digest,
    pub supports_real_world_generalization: bool,
    pub limitation_codes: Vec<String>,
}

impl ProtectedSliceManifestV2 {
    #[must_use]
    pub fn recompute_digest(&self) -> Sha256Digest {
        let mut canonical = self.clone();
        canonical.member_ids.sort();
        canonical.held_out_from_train_by.sort();
        canonical.limitation_codes.sort();
        canonical.artifact_digest = Sha256Digest::of_bytes(b"placeholder");
        Sha256Digest::of_bytes(&serde_json::to_vec(&canonical).expect("split serialization"))
    }
}

/// Additive, backward-readable protected-corpus manifest. V1 remains accepted unchanged.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtectedCorpusManifestV2 {
    pub schema_version: String,
    pub corpus_digest: Sha256Digest,
    pub datasheet_revision: String,
    pub foundry_revision: String,
    pub ontology_snapshot_digest: Sha256Digest,
    pub calibration_snapshot_digest: Sha256Digest,
    pub builder_configuration_digest: Sha256Digest,
    pub inputs: Vec<CorpusInputBinding>,
    pub members: Vec<CorpusMemberV2>,
    pub exclusions: Vec<CorpusExclusion>,
    pub slices: Vec<ProtectedSliceManifestV2>,
    pub contract_plumbing_ready: bool,
    pub model_qualification_ready: bool,
    pub limitation_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorpusContractError {
    Missing(&'static str),
    Invalid(&'static str),
    Duplicate(&'static str),
    Leakage(&'static str),
}

impl std::fmt::Display for CorpusContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CorpusContractError {}

impl ProtectedCorpusManifest {
    #[must_use]
    pub fn recompute_digest(&self) -> Sha256Digest {
        let mut canonical = self.clone();
        canonical.corpus_digest = Sha256Digest::of_bytes(b"placeholder");
        canonical
            .inputs
            .sort_by(|left, right| left.role.cmp(&right.role));
        canonical
            .members
            .sort_by(|left, right| left.example_id.cmp(&right.example_id));
        canonical
            .exclusions
            .sort_by(|left, right| left.example_id.cmp(&right.example_id));
        canonical.splits.sort_by_key(|split| split.split);
        for member in &mut canonical.members {
            member.lineage_group_ids.sort();
            member.rights_snapshot_ids.sort();
            member.restriction_codes.sort();
        }
        for exclusion in &mut canonical.exclusions {
            exclusion.reason_codes.sort();
            exclusion.rights_snapshot_ids.sort();
        }
        for split in &mut canonical.splits {
            split.member_ids.sort();
            split.limitation_codes.sort();
        }
        canonical.limitation_codes.sort();
        Sha256Digest::of_bytes(&serde_json::to_vec(&canonical).expect("corpus serialization"))
    }

    pub fn validate(&self) -> Result<(), CorpusContractError> {
        if self.schema_version != PROTECTED_CORPUS_SCHEMA
            || self.datasheet_revision.trim().is_empty()
            || self.foundry_revision.trim().is_empty()
            || self.inputs.is_empty()
            || self.members.is_empty()
        {
            return Err(CorpusContractError::Missing("corpus identity or members"));
        }
        for digest in [
            &self.corpus_digest,
            &self.ontology_snapshot_digest,
            &self.calibration_snapshot_digest,
            &self.builder_configuration_digest,
        ] {
            digest
                .validate()
                .map_err(|_| CorpusContractError::Invalid("digest"))?;
        }
        unique(self.inputs.iter().map(|input| &input.role), "input")?;
        for input in &self.inputs {
            input
                .content_sha256
                .validate()
                .map_err(|_| CorpusContractError::Invalid("input digest"))?;
            if input.role.trim().is_empty() || input.row_count == 0 {
                return Err(CorpusContractError::Missing("input identity or rows"));
            }
        }
        let input_rows = self.inputs.iter().try_fold(0_u64, |total, input| {
            total
                .checked_add(input.row_count)
                .ok_or(CorpusContractError::Invalid("input row count overflow"))
        })?;
        if input_rows
            != u64::try_from(self.members.len() + self.exclusions.len()).unwrap_or(u64::MAX)
        {
            return Err(CorpusContractError::Invalid("input/member cardinality"));
        }
        unique(
            self.members.iter().map(|member| &member.example_id),
            "member",
        )?;
        unique(
            self.exclusions.iter().map(|item| &item.example_id),
            "exclusion",
        )?;
        unique(self.splits.iter().map(|split| split.split), "split")?;
        if self.splits.len() != ProtectedSplit::ALL.len()
            || !ProtectedSplit::ALL
                .iter()
                .all(|expected| self.splits.iter().any(|split| split.split == *expected))
        {
            return Err(CorpusContractError::Missing("all six protected splits"));
        }
        let excluded = self
            .exclusions
            .iter()
            .map(|item| &item.example_id)
            .collect::<BTreeSet<_>>();
        for item in &self.exclusions {
            item.example_id
                .validate()
                .map_err(|_| CorpusContractError::Invalid("exclusion digest"))?;
            if item.reason_codes.is_empty() || item.rights_snapshot_ids.is_empty() {
                return Err(CorpusContractError::Missing("exclusion reason or rights"));
            }
            unique(item.reason_codes.iter(), "exclusion reason")?;
            unique(item.rights_snapshot_ids.iter(), "exclusion rights")?;
            if item
                .reason_codes
                .iter()
                .any(|reason| reason.trim().is_empty())
            {
                return Err(CorpusContractError::Missing("exclusion reason"));
            }
            for digest in &item.rights_snapshot_ids {
                digest
                    .validate()
                    .map_err(|_| CorpusContractError::Invalid("exclusion rights digest"))?;
            }
        }
        if self
            .members
            .iter()
            .any(|member| excluded.contains(&member.example_id))
        {
            return Err(CorpusContractError::Invalid("excluded example admitted"));
        }
        for member in &self.members {
            validate_member(member)?;
        }
        let by_id = self
            .members
            .iter()
            .map(|member| (&member.example_id, member))
            .collect::<BTreeMap<_, _>>();
        for split in &self.splits {
            split
                .artifact_digest
                .validate()
                .map_err(|_| CorpusContractError::Invalid("split artifact digest"))?;
            if split.artifact_digest != split.recompute_digest() {
                return Err(CorpusContractError::Invalid("split artifact identity"));
            }
            unique(split.member_ids.iter(), "split member")?;
            if split.split == ProtectedSplit::FinalBlindTest {
                if !split.hidden {
                    return Err(CorpusContractError::Invalid(
                        "final blind split is not hidden",
                    ));
                }
            } else if split.hidden {
                return Err(CorpusContractError::Invalid(
                    "non-blind split marked hidden",
                ));
            }
            if split.member_ids.is_empty() && split.limitation_codes.is_empty() {
                return Err(CorpusContractError::Missing("empty split limitation"));
            }
            for id in &split.member_ids {
                let member = by_id
                    .get(id)
                    .ok_or(CorpusContractError::Missing("split member"))?;
                if member.assigned_split != split.split {
                    return Err(CorpusContractError::Invalid("split projection"));
                }
            }
        }
        for member in &self.members {
            if self
                .splits
                .iter()
                .filter(|split| split.member_ids.contains(&member.example_id))
                .count()
                != 1
            {
                return Err(CorpusContractError::Invalid("member split cardinality"));
            }
            if let Some(root) = &member.synthetic_root_example_id {
                let root_member = by_id
                    .get(root)
                    .ok_or(CorpusContractError::Missing("synthetic root member"))?;
                if root_member.assigned_split != member.assigned_split {
                    return Err(CorpusContractError::Leakage(
                        "synthetic derivative crosses split",
                    ));
                }
            }
        }
        reject_group_leakage(&self.members)?;
        let all_evaluation_nonempty = self
            .splits
            .iter()
            .filter(|split| split.split != ProtectedSplit::Train)
            .all(|split| !split.member_ids.is_empty());
        let grouping_complete = self.members.iter().all(|member| {
            member
                .template_family_id
                .as_deref()
                .is_some_and(|group| !group.is_empty())
                && member
                    .manufacturer_key
                    .as_deref()
                    .is_some_and(|group| !group.is_empty())
        });
        let ready = all_evaluation_nonempty && self.exclusions.is_empty() && grouping_complete;
        if self.model_qualification_ready != ready {
            return Err(CorpusContractError::Invalid(
                "model qualification readiness",
            ));
        }
        if !ready && self.limitation_codes.is_empty() {
            return Err(CorpusContractError::Missing("corpus limitations"));
        }
        if self.corpus_digest != self.recompute_digest() {
            return Err(CorpusContractError::Invalid("corpus digest"));
        }
        Ok(())
    }
}

fn validate_member(member: &CorpusMember) -> Result<(), CorpusContractError> {
    if member.document_family_id.trim().is_empty()
        || member.lineage_group_ids.is_empty()
        || member.rights_snapshot_ids.is_empty()
        || !member.training_permitted
        || !member.restriction_codes.is_empty()
    {
        return Err(CorpusContractError::Invalid(
            "untrainable or incomplete member",
        ));
    }
    for digest in [
        &member.example_id,
        &member.example_content_sha256,
        &member.near_duplicate_group_id,
    ] {
        digest
            .validate()
            .map_err(|_| CorpusContractError::Invalid("member digest"))?;
    }
    for digest in member
        .lineage_group_ids
        .iter()
        .chain(&member.rights_snapshot_ids)
        .chain(member.revision_group_id.iter())
        .chain(member.synthetic_root_example_id.iter())
    {
        digest
            .validate()
            .map_err(|_| CorpusContractError::Invalid("member grouping digest"))?;
    }
    if member
        .template_family_id
        .iter()
        .chain(&member.manufacturer_key)
        .any(|group| group.trim().is_empty())
    {
        return Err(CorpusContractError::Missing("member grouping key"));
    }
    unique(member.lineage_group_ids.iter(), "lineage")?;
    unique(member.rights_snapshot_ids.iter(), "rights")?;
    if member.assigned_split == ProtectedSplit::TemplateHeldOut
        && member
            .template_family_id
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err(CorpusContractError::Missing("template-held-out group"));
    }
    if member.assigned_split == ProtectedSplit::ManufacturerHeldOut
        && member.manufacturer_key.as_deref().is_none_or(str::is_empty)
    {
        return Err(CorpusContractError::Missing("manufacturer-held-out group"));
    }
    Ok(())
}

fn reject_group_leakage(members: &[CorpusMember]) -> Result<(), CorpusContractError> {
    let mut groups = BTreeMap::<String, ProtectedSplit>::new();
    for member in members {
        let mut keys = vec![
            format!("document:{}", member.document_family_id),
            format!("content:{}", member.example_content_sha256.0),
            format!("near:{}", member.near_duplicate_group_id.0),
        ];
        keys.extend(
            member
                .lineage_group_ids
                .iter()
                .map(|id| format!("lineage:{}", id.0)),
        );
        if let Some(group) = &member.revision_group_id {
            keys.push(format!("revision:{}", group.0));
        }
        if let Some(root) = &member.synthetic_root_example_id {
            keys.push(format!("synthetic:{}", root.0));
        }
        if let Some(group) = &member.template_family_id {
            keys.push(format!("template:{group}"));
        }
        if let Some(group) = &member.manufacturer_key {
            keys.push(format!("manufacturer:{group}"));
        }
        for key in keys {
            if let Some(prior) = groups.insert(key, member.assigned_split) {
                if prior != member.assigned_split {
                    return Err(CorpusContractError::Leakage(
                        "protected group crosses splits",
                    ));
                }
            }
        }
    }
    Ok(())
}

impl ProtectedCorpusManifestV2 {
    #[must_use]
    pub fn recompute_digest(&self) -> Sha256Digest {
        let mut canonical = self.clone();
        canonical.corpus_digest = Sha256Digest::of_bytes(b"placeholder");
        canonical.inputs.sort_by(|a, b| a.role.cmp(&b.role));
        canonical
            .members
            .sort_by(|a, b| a.example_id.cmp(&b.example_id));
        canonical
            .exclusions
            .sort_by(|a, b| a.example_id.cmp(&b.example_id));
        canonical.slices.sort_by_key(|slice| slice.split);
        for member in &mut canonical.members {
            member.lineage_group_ids.sort();
            member.rights_snapshot_ids.sort();
            member.restriction_codes.sort();
        }
        for exclusion in &mut canonical.exclusions {
            exclusion.reason_codes.sort();
            exclusion.rights_snapshot_ids.sort();
        }
        for slice in &mut canonical.slices {
            slice.member_ids.sort();
            slice.held_out_from_train_by.sort();
            slice.limitation_codes.sort();
        }
        canonical.limitation_codes.sort();
        Sha256Digest::of_bytes(&serde_json::to_vec(&canonical).expect("corpus serialization"))
    }

    pub fn validate(&self) -> Result<(), CorpusContractError> {
        if self.schema_version != PROTECTED_CORPUS_V2_SCHEMA
            || self.datasheet_revision.trim().is_empty()
            || self.foundry_revision.trim().is_empty()
            || self.inputs.is_empty()
            || self.members.is_empty()
        {
            return Err(CorpusContractError::Missing("corpus identity or members"));
        }
        for digest in [
            &self.corpus_digest,
            &self.ontology_snapshot_digest,
            &self.calibration_snapshot_digest,
            &self.builder_configuration_digest,
        ] {
            digest
                .validate()
                .map_err(|_| CorpusContractError::Invalid("digest"))?;
        }
        unique(self.inputs.iter().map(|input| &input.role), "input")?;
        let input_rows = self.inputs.iter().try_fold(0_u64, |total, input| {
            input
                .content_sha256
                .validate()
                .map_err(|_| CorpusContractError::Invalid("input digest"))?;
            if input.role.trim().is_empty() || input.row_count == 0 {
                return Err(CorpusContractError::Missing("input identity or rows"));
            }
            total
                .checked_add(input.row_count)
                .ok_or(CorpusContractError::Invalid("input row count overflow"))
        })?;
        if input_rows
            != u64::try_from(self.members.len() + self.exclusions.len()).unwrap_or(u64::MAX)
        {
            return Err(CorpusContractError::Invalid("input/member cardinality"));
        }
        unique(
            self.members.iter().map(|member| &member.example_id),
            "member",
        )?;
        unique(
            self.exclusions.iter().map(|item| &item.example_id),
            "exclusion",
        )?;
        unique(self.slices.iter().map(|slice| slice.split), "split")?;
        if self.slices.len() != ProtectedSplit::ALL.len()
            || !ProtectedSplit::ALL
                .iter()
                .all(|expected| self.slices.iter().any(|slice| slice.split == *expected))
        {
            return Err(CorpusContractError::Missing("all six protected splits"));
        }
        let excluded = self
            .exclusions
            .iter()
            .map(|item| &item.example_id)
            .collect::<BTreeSet<_>>();
        for item in &self.exclusions {
            item.example_id
                .validate()
                .map_err(|_| CorpusContractError::Invalid("exclusion digest"))?;
            if item.reason_codes.is_empty() || item.rights_snapshot_ids.is_empty() {
                return Err(CorpusContractError::Missing("exclusion reason or rights"));
            }
            unique(item.reason_codes.iter(), "exclusion reason")?;
            unique(item.rights_snapshot_ids.iter(), "exclusion rights")?;
            for digest in &item.rights_snapshot_ids {
                digest
                    .validate()
                    .map_err(|_| CorpusContractError::Invalid("exclusion rights digest"))?;
            }
        }
        if self
            .members
            .iter()
            .any(|member| excluded.contains(&member.example_id))
        {
            return Err(CorpusContractError::Invalid("excluded example admitted"));
        }
        for member in &self.members {
            validate_member_v2(member)?;
        }
        let by_id = self
            .members
            .iter()
            .map(|m| (&m.example_id, m))
            .collect::<BTreeMap<_, _>>();
        let train = self
            .slices
            .iter()
            .find(|s| s.split == ProtectedSplit::Train)
            .expect("six slices");
        let train_ids = train.member_ids.iter().collect::<BTreeSet<_>>();
        for slice in &self.slices {
            slice
                .artifact_digest
                .validate()
                .map_err(|_| CorpusContractError::Invalid("split artifact digest"))?;
            if slice.artifact_digest != slice.recompute_digest() {
                return Err(CorpusContractError::Invalid("split artifact identity"));
            }
            unique(slice.member_ids.iter(), "split member")?;
            unique(slice.held_out_from_train_by.iter(), "held-out grouping")?;
            if (slice.split == ProtectedSplit::FinalBlindTest) != slice.hidden {
                return Err(CorpusContractError::Invalid("split hidden state"));
            }
            if slice.member_ids.is_empty() && slice.limitation_codes.is_empty() {
                return Err(CorpusContractError::Missing("empty split limitation"));
            }
            if slice.split == ProtectedSplit::Train {
                if !slice.held_out_from_train_by.is_empty() {
                    return Err(CorpusContractError::Invalid("train held-out declaration"));
                }
            } else {
                let mandatory = [
                    GroupingLevel::ExactContent,
                    GroupingLevel::NearDuplicate,
                    GroupingLevel::Revision,
                    GroupingLevel::SyntheticRoot,
                ];
                if !mandatory
                    .iter()
                    .all(|level| slice.held_out_from_train_by.contains(level))
                {
                    return Err(CorpusContractError::Missing(
                        "baseline train-disjoint group",
                    ));
                }
                let named = match slice.split {
                    ProtectedSplit::TemplateHeldOut => Some(GroupingLevel::TemplateFamily),
                    ProtectedSplit::DocumentFamilyHeldOut => Some(GroupingLevel::DocumentFamily),
                    ProtectedSplit::ManufacturerHeldOut => Some(GroupingLevel::Manufacturer),
                    _ => None,
                };
                if named.is_some_and(|level| !slice.held_out_from_train_by.contains(&level)) {
                    return Err(CorpusContractError::Missing("named held-out group"));
                }
            }
            for id in &slice.member_ids {
                if !by_id.contains_key(id) {
                    return Err(CorpusContractError::Missing("split member"));
                }
                if slice.split != ProtectedSplit::Train && train_ids.contains(id) {
                    return Err(CorpusContractError::Leakage(
                        "member tagged train and evaluation",
                    ));
                }
            }
            if slice.supports_real_world_generalization {
                if slice.member_ids.is_empty()
                    || slice
                        .member_ids
                        .iter()
                        .any(|id| !by_id[id].real_world_generalization_eligible)
                {
                    return Err(CorpusContractError::Invalid(
                        "unsupported real-world generalization claim",
                    ));
                }
            } else if slice.split != ProtectedSplit::Train
                && !slice.member_ids.is_empty()
                && slice.limitation_codes.is_empty()
            {
                return Err(CorpusContractError::Missing(
                    "non-generalization limitation",
                ));
            }
            if slice.split != ProtectedSplit::Train {
                reject_slice_train_leakage(slice, train, &by_id)?;
            }
        }
        for member in &self.members {
            if !self
                .slices
                .iter()
                .any(|slice| slice.member_ids.contains(&member.example_id))
            {
                return Err(CorpusContractError::Missing("member slice tag"));
            }
        }
        let plumbing_ready = self.slices.iter().all(|slice| !slice.member_ids.is_empty());
        if self.contract_plumbing_ready != plumbing_ready {
            return Err(CorpusContractError::Invalid("contract plumbing readiness"));
        }
        let qualification_ready = plumbing_ready
            && self.exclusions.is_empty()
            && self
                .slices
                .iter()
                .filter(|s| s.split != ProtectedSplit::Train)
                .all(|s| s.supports_real_world_generalization);
        if self.model_qualification_ready != qualification_ready {
            return Err(CorpusContractError::Invalid(
                "model qualification readiness",
            ));
        }
        if !qualification_ready && self.limitation_codes.is_empty() {
            return Err(CorpusContractError::Missing("corpus limitations"));
        }
        if self.corpus_digest != self.recompute_digest() {
            return Err(CorpusContractError::Invalid("corpus digest"));
        }
        Ok(())
    }
}

fn validate_member_v2(member: &CorpusMemberV2) -> Result<(), CorpusContractError> {
    if member.document_family_id.trim().is_empty()
        || member.lineage_group_ids.is_empty()
        || member.rights_snapshot_ids.is_empty()
        || !member.training_permitted
        || !member.restriction_codes.is_empty()
    {
        return Err(CorpusContractError::Invalid(
            "untrainable or incomplete member",
        ));
    }
    if member.evidence_class != CorpusEvidenceClass::RealDocument
        && member.real_world_generalization_eligible
    {
        return Err(CorpusContractError::Invalid(
            "synthetic generalization eligibility",
        ));
    }
    if member.evidence_class == CorpusEvidenceClass::SyntheticDerivative
        && member.synthetic_root_example_id.is_none()
    {
        return Err(CorpusContractError::Missing("synthetic derivative root"));
    }
    for digest in [
        &member.example_id,
        &member.example_content_sha256,
        &member.near_duplicate_group_id,
    ]
    .into_iter()
    .chain(&member.lineage_group_ids)
    .chain(&member.rights_snapshot_ids)
    .chain(member.revision_group_id.iter())
    .chain(member.synthetic_root_example_id.iter())
    {
        digest
            .validate()
            .map_err(|_| CorpusContractError::Invalid("member digest"))?;
    }
    unique(member.lineage_group_ids.iter(), "lineage")?;
    unique(member.rights_snapshot_ids.iter(), "rights")?;
    Ok(())
}

fn reject_slice_train_leakage(
    slice: &ProtectedSliceManifestV2,
    train: &ProtectedSliceManifestV2,
    by_id: &BTreeMap<&Sha256Digest, &CorpusMemberV2>,
) -> Result<(), CorpusContractError> {
    for id in train.member_ids.iter().chain(&slice.member_ids) {
        let member = by_id[id];
        for level in &slice.held_out_from_train_by {
            let complete = match level {
                GroupingLevel::Revision => member.revision_group_id.is_some(),
                GroupingLevel::TemplateFamily => member.template_family_id.is_some(),
                GroupingLevel::Manufacturer => member.manufacturer_key.is_some(),
                _ => true,
            };
            if !complete {
                return Err(CorpusContractError::Missing("declared grouping metadata"));
            }
        }
    }
    for evaluation_id in &slice.member_ids {
        let evaluation = by_id[evaluation_id];
        for train_id in &train.member_ids {
            let training = by_id[train_id];
            for level in &slice.held_out_from_train_by {
                let overlap = match level {
                    GroupingLevel::ExactContent => {
                        evaluation.example_content_sha256 == training.example_content_sha256
                    }
                    GroupingLevel::NearDuplicate => {
                        evaluation.near_duplicate_group_id == training.near_duplicate_group_id
                    }
                    GroupingLevel::Revision => {
                        option_overlap(&evaluation.revision_group_id, &training.revision_group_id)
                    }
                    GroupingLevel::SyntheticRoot => synthetic_roots(evaluation)
                        .intersection(&synthetic_roots(training))
                        .next()
                        .is_some(),
                    GroupingLevel::DocumentFamily => {
                        evaluation.document_family_id == training.document_family_id
                    }
                    GroupingLevel::TemplateFamily => {
                        option_overlap(&evaluation.template_family_id, &training.template_family_id)
                    }
                    GroupingLevel::Manufacturer => {
                        option_overlap(&evaluation.manufacturer_key, &training.manufacturer_key)
                    }
                    GroupingLevel::PublisherLineage => evaluation
                        .lineage_group_ids
                        .iter()
                        .any(|id| training.lineage_group_ids.contains(id)),
                };
                if overlap {
                    return Err(CorpusContractError::Leakage(
                        "declared group overlaps train",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn option_overlap<T: PartialEq>(left: &Option<T>, right: &Option<T>) -> bool {
    left.as_ref()
        .zip(right.as_ref())
        .is_some_and(|(left, right)| left == right)
}

fn synthetic_roots(member: &CorpusMemberV2) -> BTreeSet<&Sha256Digest> {
    let mut roots = BTreeSet::new();
    // Every example is the root of its own derivative family, including real examples
    // from which policy-permitted synthetic derivatives may later be generated.
    roots.insert(&member.example_id);
    if let Some(root) = &member.synthetic_root_example_id {
        roots.insert(root);
    }
    roots
}

fn unique<T: Ord>(
    values: impl IntoIterator<Item = T>,
    what: &'static str,
) -> Result<(), CorpusContractError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(CorpusContractError::Duplicate(what));
        }
    }
    Ok(())
}
