use std::{fs, path::Path};

use datasheet_compiler::corpus::freeze_sealed_hidden_handoff;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusEvidenceClass, GroupingLevel, ProtectedSplit, SEALED_HIDDEN_HANDOFF_SCHEMA,
    SealedHandoffState, SealedHiddenEvaluationHandoff,
};

fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: build_fdx4_sealed_handoff <output>");
    let handoff = freeze_sealed_hidden_handoff(SealedHiddenEvaluationHandoff {
        schema_version: SEALED_HIDDEN_HANDOFF_SCHEMA.into(),
        handoff_digest: Sha256Digest::of_bytes(b"placeholder"),
        qualification_context_digest: Sha256Digest::of_bytes(
            &serde_json::to_vec(&(
                "fdx4-real-group-expansion",
                "foundry-89bdce4",
                "datasheet-ff0925b",
                "external-seal-required",
            ))
            .unwrap(),
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
    fs::write(Path::new(&output), bytes).unwrap();
}
