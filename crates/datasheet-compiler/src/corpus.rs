// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic FDX4 protected-corpus construction.

use datasheet_core::corpus::{ProtectedCorpusManifest, ProtectedSplitManifest};

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

#[must_use]
pub fn split_digest(split: &ProtectedSplitManifest) -> datasheet_core::Sha256Digest {
    split.recompute_digest()
}
