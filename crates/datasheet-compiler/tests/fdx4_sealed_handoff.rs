use datasheet_compiler::corpus::freeze_sealed_hidden_handoff;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusEvidenceClass, GroupingLevel, ProtectedSplit, SEALED_HIDDEN_HANDOFF_SCHEMA,
    SealedHandoffState, SealedHiddenEvaluationHandoff,
};

fn digest(byte: char) -> Sha256Digest {
    Sha256Digest(std::iter::repeat_n(byte, 64).collect())
}

fn request() -> SealedHiddenEvaluationHandoff {
    SealedHiddenEvaluationHandoff {
        schema_version: SEALED_HIDDEN_HANDOFF_SCHEMA.into(),
        handoff_digest: digest('0'),
        qualification_context_digest: digest('1'),
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
        limitation_codes: vec!["external_operator_and_sealed_artifact_required".into()],
    }
}

#[test]
fn pending_request_replays_but_is_not_hidden_ready() {
    let frozen = freeze_sealed_hidden_handoff(request()).unwrap();
    assert!(!frozen.ready_for_hidden_evaluation());
    let mut reordered = frozen.clone();
    reordered.held_out_from_train_by.reverse();
    assert_eq!(freeze_sealed_hidden_handoff(reordered).unwrap(), frozen);
}

#[test]
fn sealed_ready_requires_complete_external_attestation() {
    let mut incomplete = request();
    incomplete.state = SealedHandoffState::SealedReady;
    assert!(freeze_sealed_hidden_handoff(incomplete).is_err());

    let mut complete = request();
    complete.state = SealedHandoffState::SealedReady;
    complete.sealed_manifest_content_sha256 = Some(digest('2'));
    complete.artifactum_collection_id = Some(format!("sha256:{}", digest('3').0));
    complete.operator_attestation_sha256 = Some(digest('4'));
    assert!(
        freeze_sealed_hidden_handoff(complete)
            .unwrap()
            .ready_for_hidden_evaluation()
    );
}

#[test]
fn local_or_candidate_visible_handoffs_fail_closed() {
    for mutation in [
        "local",
        "source",
        "labels",
        "network",
        "credentials",
        "synthetic",
    ] {
        let mut handoff = request();
        match mutation {
            "local" => handoff.external_store_locator = "file:///projects/hidden".into(),
            "source" => handoff.source_bytes_repository_visible = true,
            "labels" => handoff.labels_repository_visible = true,
            "network" => handoff.candidate_network_access = true,
            "credentials" => handoff.candidate_credentials_available = true,
            "synthetic" => {
                handoff.required_evidence_class = CorpusEvidenceClass::IndependentlySynthetic
            }
            _ => unreachable!(),
        }
        assert!(freeze_sealed_hidden_handoff(handoff).is_err(), "{mutation}");
    }
}

#[test]
fn checked_schema_matches_contract() {
    let generated =
        serde_json::to_value(schemars::schema_for!(SealedHiddenEvaluationHandoff)).unwrap();
    let checked: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/fdx4/sealed-hidden-evaluation-handoff-v1.schema.json"
    ))
    .unwrap();
    assert_eq!(generated, checked);
}
