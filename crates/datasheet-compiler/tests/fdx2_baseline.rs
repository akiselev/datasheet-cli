// SPDX-License-Identifier: MIT OR Apache-2.0

use datasheet_compiler::baseline::{
    ColumnRole, Fdx2Document, InferenceStatus, RowExtractionStatus, compile_analog_power_baseline,
    infer_header_roles, infer_regime, retrieve_predicates,
};
use datasheet_compiler::evaluation::{BaselineEvaluation, BaselineTruth, evaluate_baseline};
use datasheet_compiler::quantity::{FDX2_REGISTRY_ID, QuantityParser};
use datasheet_core::{
    PredicateVocabularySnapshot, ProducerIdentity, Sha256Digest, SpecificationRegime, ValueRole,
};

const DOCUMENT: &str = include_str!("../../../tests/fixtures/fdx2_synthetic_analog_power.json");
const VOCABULARY: &str = include_str!("../../../tests/fixtures/fdx2_predicate_vocabulary.json");
const TRUTH: &str = include_str!("../../../tests/fixtures/fdx2_synthetic_truth.json");
const GOLDEN: &str = include_str!("../../../tests/goldens/fdx2_synthetic_baseline.json");
const METRICS: &str = include_str!("../../../tests/goldens/fdx2_synthetic_metrics.json");

fn document() -> Fdx2Document {
    serde_json::from_str(DOCUMENT).unwrap()
}

fn vocabulary() -> PredicateVocabularySnapshot {
    serde_json::from_str(VOCABULARY).unwrap()
}

fn producer() -> ProducerIdentity {
    ProducerIdentity {
        name: "datasheet-compiler".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        code_revision: "fdx2-frozen-baseline-v1".into(),
        configuration_digest: Sha256Digest(
            "4444444444444444444444444444444444444444444444444444444444444444".into(),
        ),
        document_ir_schema: "ferrodoc-document-ir/1".into(),
        ontology_snapshot_digest: vocabulary().digest,
        quantitas_registry_identity: Some(FDX2_REGISTRY_ID.into()),
        model_digest: None,
    }
}

fn baseline() -> datasheet_compiler::baseline::DeterministicBaseline {
    compile_analog_power_baseline(&document(), producer(), &vocabulary(), Some("analog_power"))
        .unwrap()
}

#[test]
fn frozen_baseline_and_metrics_are_byte_stable() {
    let first = baseline();
    let second = baseline();
    assert_eq!(first, second);
    assert_eq!(serde_json::to_string_pretty(&first).unwrap() + "\n", GOLDEN);
    let truth: BaselineTruth = serde_json::from_str(TRUTH).unwrap();
    let metrics = evaluate_baseline(&document(), &first, &truth);
    assert_eq!(
        serde_json::to_string_pretty(&metrics).unwrap() + "\n",
        METRICS
    );
    assert_eq!(metrics.section_precision.value, 1.0);
    assert_eq!(metrics.section_recall.value, 1.0);
    assert_eq!(metrics.header_role_accuracy.value, 1.0);
    assert_eq!(metrics.quantity_parse_accuracy.value, 1.0);
    assert_eq!(metrics.predicate_top1_accuracy.value, 1.0);
    assert_eq!(metrics.predicate_top5_accuracy.value, 1.0);
    assert_eq!(metrics.condition_span_accuracy.value, 1.0);
    assert_eq!(metrics.exact_claim_accuracy.value, 1.0);
    assert_eq!(metrics.claim_anchor_validity.value, 1.0);
    assert_eq!(metrics.ungrounded_output_count, 0);
}

#[test]
fn all_required_regimes_and_value_roles_remain_distinct() {
    let baseline = baseline();
    for regime in [
        SpecificationRegime::RecommendedOperating,
        SpecificationRegime::Guaranteed,
        SpecificationRegime::AbsoluteMaximum,
        SpecificationRegime::Informative,
    ] {
        assert!(
            baseline
                .sketch
                .sections
                .iter()
                .any(|section| { section.deterministic_regime_candidates.contains(&regime) })
        );
    }
    let input_roles: Vec<_> = baseline
        .row_results
        .iter()
        .filter(|row| {
            row.parameter_text == "Input voltage"
                && row.regime == SpecificationRegime::RecommendedOperating
        })
        .map(|row| row.value_role.clone())
        .collect();
    assert_eq!(
        input_roles,
        vec![ValueRole::Minimum, ValueRole::Typical, ValueRole::Maximum]
    );
    assert!(infer_regime("Characteristics").is_none());
}

#[test]
fn ambiguous_headers_abstain_and_missing_cells_never_become_zero() {
    let document = document();
    let table = document.pages[0]
        .tables
        .iter()
        .find(|table| table.region_id == "table-ambiguous")
        .unwrap();
    let inference = infer_header_roles(table);
    assert_eq!(inference.status, InferenceStatus::Abstained);
    assert_eq!(
        inference.roles,
        vec![ColumnRole::Parameter, ColumnRole::Unknown, ColumnRole::Unit]
    );
    let baseline = baseline();
    let missing: Vec<_> = baseline
        .row_results
        .iter()
        .filter(|row| row.parameter_text == "Output accuracy" && row.value_candidate_id.is_none())
        .collect();
    assert_eq!(missing.len(), 2);
    assert!(
        missing
            .iter()
            .all(|row| row.status == RowExtractionStatus::Unsupported)
    );
}

#[test]
fn inherited_units_are_deterministic_and_conflicts_refuse() {
    let baseline = baseline();
    let minimum = baseline
        .row_results
        .iter()
        .find(|row| row.parameter_text == "Input voltage" && row.value_role == ValueRole::Minimum)
        .unwrap();
    assert!(
        minimum
            .explanations
            .iter()
            .any(|reason| reason.contains("inherited"))
    );

    let mut conflicting = document();
    let cell = conflicting.pages[0].tables[0]
        .cells
        .iter_mut()
        .find(|cell| cell.evidence_id == "rec-13")
        .unwrap();
    cell.text = "1.8 nA".into();
    let compiled = compile_analog_power_baseline(
        &conflicting,
        producer(),
        &vocabulary(),
        Some("analog_power"),
    )
    .unwrap();
    let row = compiled
        .row_results
        .iter()
        .find(|row| row.parameter_text == "Input voltage" && row.value_role == ValueRole::Minimum)
        .unwrap();
    assert_eq!(row.status, RowExtractionStatus::Ambiguous);
    assert!(row.value_candidate_id.is_none());
}

#[test]
fn conditions_and_footnotes_are_table_scoped_and_anchored() {
    let fixture_document = document();
    let baseline = baseline();
    let row = baseline
        .row_results
        .iter()
        .find(|row| row.parameter_text == "Input voltage" && row.value_role == ValueRole::Minimum)
        .unwrap();
    let texts: Vec<_> = row
        .condition_candidate_ids
        .iter()
        .map(|id| {
            baseline
                .sketch
                .conditions
                .iter()
                .find(|condition| &condition.id == id)
                .unwrap()
                .raw_text
                .as_str()
        })
        .collect();
    assert!(texts.contains(&"TA = 25 °C"));
    assert!(texts.contains(&"Valid for the QFN-8 package only."));
    for condition in &baseline.sketch.conditions {
        for anchor in &condition.evidence_anchors {
            fixture_document.validate_anchor(anchor).unwrap();
        }
    }

    let mut ambiguous = document();
    let mut duplicate = ambiguous.pages[0].tables[0].footnotes[0].clone();
    duplicate.evidence.evidence_id = "rec-foot-1-duplicate".into();
    duplicate.evidence.text = "A conflicting interpretation.".into();
    ambiguous.pages[0].tables[0].footnotes.push(duplicate);
    let compiled =
        compile_analog_power_baseline(&ambiguous, producer(), &vocabulary(), Some("analog_power"))
            .unwrap();
    let row = compiled
        .row_results
        .iter()
        .find(|row| row.parameter_text == "Input voltage" && row.value_role == ValueRole::Minimum)
        .unwrap();
    assert_eq!(row.condition_candidate_ids.len(), 1);
    assert!(
        row.explanations
            .iter()
            .any(|reason| reason.contains("table-local scope was ambiguous"))
    );
}

#[test]
fn predicate_retrieval_filters_impossible_dimensions_and_shapes() {
    let parser = QuantityParser::fdx2().unwrap();
    let parsed = parser.parse("3.3 V", None).unwrap();
    let ranked = retrieve_predicates(
        &vocabulary(),
        "Input voltage",
        Some("VIN"),
        &parsed,
        &SpecificationRegime::RecommendedOperating,
        Some("analog_power"),
        5,
    );
    assert_eq!(
        ranked[0].predicate_term_id,
        "analog.input_voltage.operating"
    );
    assert!(
        !ranked
            .iter()
            .any(|candidate| candidate.predicate_term_id.starts_with("impossible"))
    );
    assert!(ranked[0].reasons.len() >= 4);
}

#[test]
fn every_result_explains_acceptance_or_refusal_and_all_claims_are_grounded() {
    let baseline = baseline();
    assert!(
        baseline
            .row_results
            .iter()
            .all(|row| !row.explanations.is_empty())
    );
    assert_eq!(baseline.claim_bundle.claims.len(), 7);
    assert!(
        baseline
            .claim_bundle
            .claims
            .iter()
            .all(|claim| !claim.evidence_anchors.is_empty())
    );
    assert_eq!(baseline.abstained_tables, 1);
}

#[test]
fn evaluation_counts_wrong_or_missing_predictions_as_failures() {
    let baseline = baseline();
    let mut truth: BaselineTruth = serde_json::from_str(TRUTH).unwrap();
    truth.rows[0].canonical_value = Some("wrong".into());
    truth.rows.push(truth.rows[0].clone());
    truth.rows.last_mut().unwrap().parameter_text = "Missing prediction".into();
    let metrics = evaluate_baseline(&document(), &baseline, &truth);
    assert!(metrics.quantity_parse_accuracy.value < 1.0);
    assert!(metrics.exact_claim_accuracy.value < 1.0);
}

#[test]
fn schema_snapshots_are_current() {
    let cases = [
        (
            include_str!("../../../schemas/fdx2/deterministic-baseline-v1.schema.json"),
            serde_json::to_value(schemars::schema_for!(
                datasheet_compiler::baseline::DeterministicBaseline
            ))
            .unwrap(),
        ),
        (
            include_str!("../../../schemas/fdx2/evaluation-v1.schema.json"),
            serde_json::to_value(schemars::schema_for!(BaselineEvaluation)).unwrap(),
        ),
    ];
    for (snapshot, generated) in cases {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(snapshot).unwrap(),
            generated
        );
    }
}
