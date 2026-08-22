use std::{fs, path::Path};

use datasheet_compiler::corpus::freeze_sealed_hidden_handoff;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusEvidenceClass, GroupingLevel, ProtectedSplit, SEALED_HIDDEN_HANDOFF_SCHEMA,
    SealedArtifactRole, SealedHandoffState, SealedHiddenEvaluationHandoff,
};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        3,
        "usage: build_fdx4_sealed_handoff <corpus-manifest> <output>"
    );
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(
        manifest["schema_version"],
        "fdx4.protected-corpus-manifest/v2"
    );
    assert_eq!(manifest["model_qualification_ready"], false);
    let train_ids = manifest["slices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slice| slice["split"] == "train")
        .unwrap()["member_ids"]
        .as_array()
        .unwrap();
    let train_members = manifest["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|member| train_ids.contains(&member["example_id"]))
        .cloned()
        .collect::<Vec<_>>();
    assert!(!train_members.is_empty());
    let handoff = freeze_sealed_hidden_handoff(SealedHiddenEvaluationHandoff {
        schema_version: SEALED_HIDDEN_HANDOFF_SCHEMA.into(),
        handoff_digest: Sha256Digest::of_bytes(b"placeholder"),
        qualification_context_digest: serde_json::from_value(manifest["corpus_digest"].clone())
            .unwrap(),
        training_group_commitment_sha256: Sha256Digest::of_bytes(
            &serde_json::to_vec(&train_members).unwrap(),
        ),
        split: ProtectedSplit::FinalBlindTest,
        state: SealedHandoffState::AwaitingExternalSeal,
        external_store_locator: "artifactum+sealed://independent-evaluator/fdx4/final-blind-test"
            .into(),
        held_out_from_train_by: vec![
            GroupingLevel::ExactContent,
            GroupingLevel::NearDuplicate,
            GroupingLevel::Revision,
            GroupingLevel::SyntheticRoot,
            GroupingLevel::DocumentFamily,
            GroupingLevel::TemplateFamily,
            GroupingLevel::Manufacturer,
            GroupingLevel::PublisherLineage,
        ],
        required_evidence_class: CorpusEvidenceClass::RealDocument,
        required_artifact_roles: SealedArtifactRole::ALL.into(),
        retention_permission_required: true,
        training_permission_required: true,
        policy_fail_closed: true,
        source_bytes_repository_visible: false,
        labels_repository_visible: false,
        candidate_network_access: false,
        candidate_credentials_available: false,
        sealed_manifest_content_sha256: None,
        artifactum_collection_id: None,
        operator_attestation_sha256: None,
        limitation_codes: vec![
            "external_independent_operator_not_yet_assigned".into(),
            "sealed_real_artifact_not_yet_deposited".into(),
            "final_blind_evaluation_not_claimed".into(),
        ],
    })
    .unwrap();
    let mut bytes = serde_json::to_vec_pretty(&handoff).unwrap();
    bytes.push(b'\n');
    fs::write(Path::new(&args[2]), bytes).unwrap();
}
