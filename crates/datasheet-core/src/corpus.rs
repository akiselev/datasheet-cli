// SPDX-License-Identifier: MIT OR Apache-2.0
//! Immutable FDX4 protected-corpus and split contracts.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Sha256Digest;

pub const PROTECTED_CORPUS_SCHEMA: &str = "fdx4.protected-corpus-manifest/v1";

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
