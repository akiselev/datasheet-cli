// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use datasheet_compiler::ferrodoc::{
    FERRODOC_CONFORMANCE_REVISION, FERRODOC_DOCUMENT_STATE_SCHEMA_SHA256,
    FERRODOC_EVIDENCE_DELTA_SCHEMA_SHA256, FerrodocBridgeError, FerrodocConformanceInput,
    FerrodocDocumentStateManifest, FerrodocEvidenceDelta, adapt_ferrodoc_document,
    compile_ferrodoc_analog_power_baseline,
};
use datasheet_compiler::quantity::FDX2_REGISTRY_ID;
use datasheet_core::{
    EvidenceAnchor, PredicateVocabularySnapshot, ProducerIdentity, Sha256Digest, ValueRole,
};

const INPUT: &str = include_str!("../../../tests/fixtures/ferrodoc_fdx0_datasheet_input.json");
const UPSTREAM_DELTA: &str =
    include_str!("../../../tests/fixtures/ferrodoc_upstream_evidence_delta_v1.json");
const UPSTREAM_STATE: &str =
    include_str!("../../../tests/fixtures/ferrodoc_upstream_document_state_v1.json");
const VOCABULARY: &str = include_str!("../../../tests/fixtures/fdx2_predicate_vocabulary.json");

fn input() -> FerrodocConformanceInput {
    serde_json::from_str(INPUT).unwrap()
}

fn vocabulary() -> PredicateVocabularySnapshot {
    serde_json::from_str(VOCABULARY).unwrap()
}

fn producer() -> ProducerIdentity {
    ProducerIdentity {
        name: "datasheet-compiler".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        code_revision: "ferrodoc-portable-conformance-v1".into(),
        configuration_digest: Sha256Digest(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
        ),
        document_ir_schema: "ferrodoc-document-ir/1.0".into(),
        ontology_snapshot_digest: vocabulary().digest,
        quantitas_registry_identity: Some(FDX2_REGISTRY_ID.into()),
        model_digest: None,
    }
}

#[test]
fn wire_mirror_replays_landed_ferrodoc_golden_identities() {
    let delta: FerrodocEvidenceDelta = serde_json::from_str(UPSTREAM_DELTA).unwrap();
    assert_eq!(
        delta.logical_id().unwrap(),
        "delta_a7231c4020619c1550ee2b10478f4d792c436635b413161bd89831ad43a75904"
    );
    let state: FerrodocDocumentStateManifest = serde_json::from_str(UPSTREAM_STATE).unwrap();
    assert_eq!(
        state.logical_id().unwrap(),
        "state_601dae7246e6c2773b67112c1eb5fb48269d88e80be66ca69f6c6ea98e702516"
    );
    assert_eq!(
        FERRODOC_CONFORMANCE_REVISION,
        "2a4d886703753a8893bfa17ade8a554df61f59ab"
    );
    assert_eq!(
        FERRODOC_EVIDENCE_DELTA_SCHEMA_SHA256,
        "ab98db6810db12bdf22b04e6b113bbbe4cbe43aa187702967ad1db30c1e0ff16"
    );
    assert_eq!(
        FERRODOC_DOCUMENT_STATE_SCHEMA_SHA256,
        "a9a81cd6b65acf1a256529337c985ad512d34a7b79274f48a926490c35f1aa2b"
    );
}

#[test]
fn landed_state_and_page_qualified_target_drive_datasheet_anchors() {
    let input = input();
    let state_id = input.validate().unwrap();
    assert_eq!(
        state_id,
        "state_839531833bb511d007fb3f9ca6ed8c651d0b0a1fc68e47b961e20d7ab903ee4b"
    );
    let document = adapt_ferrodoc_document(&input).unwrap();
    assert_eq!(document.document_state_id, state_id);
    assert_eq!(
        document.document_ir_logical_sha256.as_ref().unwrap().0,
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    );
    assert_eq!(document.pages.len(), 2);
    assert_eq!(document.pages[0].tables[0].cells.len(), 12);
    assert_eq!(document.pages[0].tables[0].footnotes.len(), 1);

    let baseline = compile_ferrodoc_analog_power_baseline(
        &input,
        producer(),
        &vocabulary(),
        Some("analog_power"),
    )
    .unwrap();
    assert_eq!(baseline.claim_bundle.claims.len(), 3);
    assert_eq!(
        baseline
            .claim_bundle
            .claims
            .iter()
            .map(|claim| claim.value_role.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([ValueRole::Minimum, ValueRole::Typical, ValueRole::Maximum])
    );
    for anchor in all_anchors(&baseline) {
        assert_eq!(anchor.document_state_id, state_id);
        assert!(anchor.page_id.starts_with("page_"));
        document.validate_anchor(anchor).unwrap();
    }
}

#[test]
fn conditions_values_and_claims_are_selected_not_generated() {
    let input = input();
    let baseline = compile_ferrodoc_analog_power_baseline(
        &input,
        producer(),
        &vocabulary(),
        Some("analog_power"),
    )
    .unwrap();
    let source_text: BTreeSet<_> = input
        .evidence_delta
        .new_pages
        .iter()
        .flat_map(|page| &page.regions)
        .flat_map(|region| &region.evidence)
        .filter_map(|evidence| match &evidence.content {
            datasheet_compiler::ferrodoc::FerrodocEvidenceContent::Text { text } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert!(
        baseline
            .sketch
            .quantities
            .iter()
            .all(|quantity| source_text.contains(quantity.raw_text.as_str()))
    );
    assert!(
        baseline
            .sketch
            .conditions
            .iter()
            .all(|condition| source_text.contains(condition.raw_text.as_str()))
    );
    assert_eq!(
        baseline
            .sketch
            .conditions
            .iter()
            .map(|condition| condition.raw_text.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["TA = 25 °C [1]", "[1] Valid only for the QFN-8 package."])
    );
    assert!(baseline.claim_bundle.claims.iter().all(|claim| {
        claim.evidence_anchors.iter().all(|anchor| {
            anchor.evidence_ids.iter().all(|id| {
                input.evidence_delta.new_pages.iter().any(|page| {
                    page.regions
                        .iter()
                        .any(|region| region.evidence.iter().any(|evidence| &evidence.id == id))
                })
            })
        })
    }));
}

#[test]
fn stale_wrong_state_and_wrong_page_qualified_targets_refuse() {
    let fixture = input();
    let state_id = fixture.validate().unwrap();

    let mut stale = fixture.clone();
    stale.targets[0].base_document_state_id =
        "state_0000000000000000000000000000000000000000000000000000000000000000".into();
    assert!(matches!(
        stale.validate(),
        Err(FerrodocBridgeError::StaleTarget)
    ));

    let mut unresolved_precondition = fixture.clone();
    unresolved_precondition.evidence_delta.input_state_id =
        Some("state_0000000000000000000000000000000000000000000000000000000000000000".into());
    assert!(matches!(
        unresolved_precondition.validate(),
        Err(FerrodocBridgeError::UnresolvedInputStatePrecondition)
    ));

    let mut wrong_page = fixture.clone();
    wrong_page.targets[0].page_id =
        "page_3333333333333333333333333333333333333333333333333333333333333333".into();
    assert!(matches!(
        wrong_page.validate(),
        Err(FerrodocBridgeError::WrongPageQualifiedTarget)
    ));

    let mut wrong_state = fixture.clone();
    wrong_state.document_state_manifest.source_pdf_sha256 =
        "9999999999999999999999999999999999999999999999999999999999999999".into();
    assert!(matches!(
        wrong_state.validate(),
        Err(FerrodocBridgeError::WrongState)
    ));

    let document = adapt_ferrodoc_document(&fixture).unwrap();
    let baseline = compile_ferrodoc_analog_power_baseline(
        &fixture,
        producer(),
        &vocabulary(),
        Some("analog_power"),
    )
    .unwrap();
    let mut stale_anchor = all_anchors(&baseline)[0].clone();
    stale_anchor.document_state_id = format!("{state_id}-stale");
    assert!(document.validate_anchor(&stale_anchor).is_err());
}

#[test]
fn portable_input_schema_snapshot_is_current() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/fdx2/ferrodoc-conformance-input-v1.schema.json"
    ))
    .unwrap();
    assert_eq!(
        serde_json::to_value(schemars::schema_for!(FerrodocConformanceInput)).unwrap(),
        expected
    );
}

fn all_anchors(
    baseline: &datasheet_compiler::baseline::DeterministicBaseline,
) -> Vec<&EvidenceAnchor> {
    baseline
        .sketch
        .subjects
        .iter()
        .map(|candidate| &candidate.evidence_anchor)
        .chain(
            baseline
                .sketch
                .blocks
                .iter()
                .map(|candidate| &candidate.anchor),
        )
        .chain(
            baseline
                .sketch
                .quantities
                .iter()
                .map(|candidate| &candidate.anchor),
        )
        .chain(
            baseline
                .sketch
                .conditions
                .iter()
                .flat_map(|candidate| &candidate.evidence_anchors),
        )
        .chain(
            baseline
                .sketch
                .tables
                .iter()
                .flat_map(|table| table.cells.iter().map(|cell| &cell.anchor)),
        )
        .collect()
}
