// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic FDX4 protected-corpus construction.

use datasheet_core::corpus::{
    ProtectedCorpusManifest, ProtectedCorpusManifestV2, ProtectedSliceManifestV2,
    ProtectedSplitManifest, SealedHiddenEvaluationHandoff,
};

/// Freezes split artifact identities, canonical corpus order, and the corpus digest.
/// # Errors
/// Returns an error if the resulting corpus violates rights, grouping, or split invariants.
pub fn freeze_protected_corpus(
    mut manifest: ProtectedCorpusManifest,
) -> Result<ProtectedCorpusManifest, datasheet_core::corpus::CorpusContractError> {
    manifest
        .inputs
        .sort_by(|left, right| left.role.cmp(&right.role));
    manifest
        .members
        .sort_by(|left, right| left.example_id.cmp(&right.example_id));
    manifest
        .exclusions
        .sort_by(|left, right| left.example_id.cmp(&right.example_id));
    manifest.splits.sort_by_key(|split| split.split);
    for member in &mut manifest.members {
        member.lineage_group_ids.sort();
        member.rights_snapshot_ids.sort();
        member.restriction_codes.sort();
    }
    for exclusion in &mut manifest.exclusions {
        exclusion.reason_codes.sort();
        exclusion.rights_snapshot_ids.sort();
    }
    for split in &mut manifest.splits {
        split.member_ids.sort();
        split.limitation_codes.sort();
        split.artifact_digest = split_digest(split);
    }
    manifest.limitation_codes.sort();
    manifest.corpus_digest = manifest.recompute_digest();
    manifest.validate()?;
    Ok(manifest)
}

/// Freezes a repository-safe request or an externally attested sealed handoff.
/// # Errors
/// Returns an error if the locator, isolation flags, seal state, or identities fail closed.
pub fn freeze_sealed_hidden_handoff(
    mut handoff: SealedHiddenEvaluationHandoff,
) -> Result<SealedHiddenEvaluationHandoff, datasheet_core::corpus::CorpusContractError> {
    handoff.held_out_from_train_by.sort();
    handoff.limitation_codes.sort();
    handoff.handoff_digest = handoff.recompute_digest();
    handoff.validate()?;
    Ok(handoff)
}

#[must_use]
pub fn split_digest(split: &ProtectedSplitManifest) -> datasheet_core::Sha256Digest {
    split.recompute_digest()
}

/// Freezes the additive v2 manifest while preserving overlapping evaluation tags.
/// # Errors
/// Returns an error if rights, declared train-disjoint grouping, or claim invariants fail.
pub fn freeze_protected_corpus_v2(
    mut manifest: ProtectedCorpusManifestV2,
) -> Result<ProtectedCorpusManifestV2, datasheet_core::corpus::CorpusContractError> {
    manifest.inputs.sort_by(|a, b| a.role.cmp(&b.role));
    manifest
        .members
        .sort_by(|a, b| a.example_id.cmp(&b.example_id));
    manifest
        .exclusions
        .sort_by(|a, b| a.example_id.cmp(&b.example_id));
    manifest.slices.sort_by_key(|slice| slice.split);
    for member in &mut manifest.members {
        member.lineage_group_ids.sort();
        member.rights_snapshot_ids.sort();
        member.restriction_codes.sort();
    }
    for exclusion in &mut manifest.exclusions {
        exclusion.reason_codes.sort();
        exclusion.rights_snapshot_ids.sort();
    }
    for slice in &mut manifest.slices {
        slice.member_ids.sort();
        slice.held_out_from_train_by.sort();
        slice.limitation_codes.sort();
        slice.artifact_digest = split_digest_v2(slice);
    }
    manifest.limitation_codes.sort();
    manifest.corpus_digest = manifest.recompute_digest();
    manifest.validate()?;
    Ok(manifest)
}

#[must_use]
pub fn split_digest_v2(split: &ProtectedSliceManifestV2) -> datasheet_core::Sha256Digest {
    split.recompute_digest()
}
