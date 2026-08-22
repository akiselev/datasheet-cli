// SPDX-License-Identifier: MIT OR Apache-2.0

use datasheet_compiler::{
    ClaimSelection, DatasheetSketch, FixtureDocumentIr, assemble_claim_bundle, sketch_from_fixture,
    validate_claim_bundle_against_sketch, validate_claim_bundle_against_sketch_and_vocabulary,
};
use datasheet_core::{
    CandidateId, ClaimBundle, EvidenceAnchor, ParsedQuantityShape, PredicateTerm,
    PredicateVocabularySnapshot, ProducerIdentity, Sha256Digest, SpecificationRegime, ValueRole,
};

const FIXTURE: &str = include_str!("../../../tests/fixtures/fdx1_synthetic_document_ir.json");

fn fixture() -> FixtureDocumentIr {
    serde_json::from_str(FIXTURE).unwrap()
}

fn producer() -> ProducerIdentity {
    ProducerIdentity {
        name: "datasheet-compiler".into(),
        version: "0.1.8".into(),
        code_revision: "fdx1-test-revision".into(),
        configuration_digest: Sha256Digest(
            "b79606fb3afea5bd1609ed40b622142f1c98125abcfe89a76a661b0e8e343910".into(),
        ),
        document_ir_schema: "ferrodoc-document-ir-fixture/1".into(),
        ontology_snapshot_digest: Sha256Digest(
            "c205f711aa5d877300e2cdbcb5d060be2e0de276e525c020356210df8836e8e9".into(),
        ),
        quantitas_registry_identity: Some("quantitas-synthetic-registry/1".into()),
        model_digest: None,
    }
}

fn sketch() -> DatasheetSketch {
    sketch_from_fixture(&fixture(), producer()).unwrap()
}

fn vocabulary() -> PredicateVocabularySnapshot {
    PredicateVocabularySnapshot {
        schema_version: "predicate-vocabulary/1".into(),
        digest: producer().ontology_snapshot_digest,
        terms: vec![PredicateTerm {
            id: "electrical.input_voltage.operating".into(),
            canonical_name: "Operating input voltage".into(),
            description: "Permitted input voltage under stated conditions".into(),
            aliases: vec!["Input voltage".into()],
            symbol_aliases: vec!["VIN".into()],
            expected_quantity_kind: Some("voltage".into()),
            expected_dimensionality: None,
            expected_shapes: vec![ParsedQuantityShape::Scalar],
            regime_constraints: vec![SpecificationRegime::RecommendedOperating],
            category_scope: vec!["analog_power".into()],
        }],
    }
}

fn selection(sketch: &DatasheetSketch) -> ClaimSelection {
    ClaimSelection {
        subject_candidate_id: sketch.subjects[0].id.clone(),
        predicate_term_id: "electrical.input_voltage.operating".into(),
        regime: SpecificationRegime::RecommendedOperating,
        value_role: ValueRole::Minimum,
        value_candidate_id: sketch.quantities[0].id.clone(),
        condition_candidate_ids: vec![sketch.conditions[0].id.clone()],
        package_variant_candidate_ids: vec![sketch.packages[0].id.clone()],
        extraction_confidence: 1.0,
    }
}

#[test]
fn fixture_produces_deterministic_family_free_sketch() {
    let first = sketch();
    let second = sketch();
    assert_eq!(first, second);
    assert_eq!(first.sketch_id, first.recompute_id().unwrap());
    assert_eq!(
        first.sketch_id.0,
        "datasheet-sketch:bfe0100aaf5a77818784f2a056f3c305a88f59f99f320fa08b9de67b5c2e743d"
    );
    assert_eq!(
        first.subjects[0].id.0,
        "subject:acd38eedea06f507b1b87814ae09f4c6e62aaa59949292c2b39fe3eb1e173d9c"
    );
    assert_eq!(
        first.quantities[0].id.0,
        "quantity:11bdd0839e0513948bb02b2e01dc4a26a63b99ed729df6cd81a28a24f0de24d6"
    );
    assert_eq!(
        first.conditions[0].id.0,
        "condition:3b437c43e03abd6dc5e6af22b5bb3b5a358a70e25fc8a04a25f3021ae5bab62a"
    );

    let encoded = serde_json::to_vec(&first).unwrap();
    let round_trip: DatasheetSketch = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(round_trip, first);
    let serialized = String::from_utf8(encoded).unwrap();
    assert!(!serialized.contains("document_family"));
    assert!(!serialized.contains("template_family"));
    assert!(!serialized.contains("manufacturer_family"));
}

#[test]
fn input_order_does_not_change_sketch_or_candidate_ids() {
    let expected = sketch();
    let mut reordered = fixture();
    reordered.pages.reverse();
    for page in &mut reordered.pages {
        page.regions.reverse();
        for region in &mut page.regions {
            region.evidence.reverse();
        }
    }
    assert_eq!(
        sketch_from_fixture(&reordered, producer()).unwrap(),
        expected
    );
}

#[test]
fn every_anchor_resolves_against_pinned_fixture() {
    let fixture = fixture();
    let sketch = sketch();
    for anchor in sketch
        .subjects
        .iter()
        .map(|candidate| &candidate.evidence_anchor)
        .chain(sketch.blocks.iter().map(|candidate| &candidate.anchor))
        .chain(sketch.quantities.iter().map(|candidate| &candidate.anchor))
        .chain(sketch.symbols.iter().map(|candidate| &candidate.anchor))
        .chain(sketch.packages.iter().map(|candidate| &candidate.anchor))
        .chain(
            sketch
                .conditions
                .iter()
                .flat_map(|candidate| &candidate.evidence_anchors),
        )
        .chain(
            sketch
                .sections
                .iter()
                .filter_map(|candidate| candidate.heading_anchor.as_ref()),
        )
        .chain(
            sketch
                .tables
                .iter()
                .flat_map(|table| table.cells.iter().map(|cell| &cell.anchor)),
        )
    {
        fixture.validate_anchor(anchor).unwrap();
    }
}

#[test]
fn normalization_preserves_exact_source_text_and_regimes() {
    let sketch = sketch();
    let quantity = &sketch.quantities[0];
    assert_eq!(quantity.raw_text, "1.80 V");
    assert_eq!(
        quantity
            .normalized_quantity
            .as_ref()
            .unwrap()
            .canonical_value,
        "1.8"
    );
    let regimes: Vec<_> = sketch
        .sections
        .iter()
        .flat_map(|section| &section.deterministic_regime_candidates)
        .collect();
    assert!(regimes.contains(&&SpecificationRegime::AbsoluteMaximum));
    assert!(regimes.contains(&&SpecificationRegime::RecommendedOperating));
}

#[test]
fn claim_bundle_is_deterministic_and_grounded() {
    let sketch = sketch();
    let selection = selection(&sketch);
    let first =
        assemble_claim_bundle(&sketch, &vocabulary(), std::slice::from_ref(&selection)).unwrap();
    let second = assemble_claim_bundle(&sketch, &vocabulary(), &[selection]).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.selected_values[0].raw_text, "1.80 V");
    assert_eq!(first.claims[0].condition_candidate_ids.len(), 1);
    validate_claim_bundle_against_sketch(&first, &sketch).unwrap();
}

#[test]
fn condition_and_package_selection_order_is_canonical() {
    let sketch = sketch();
    let expected = assemble_claim_bundle(&sketch, &vocabulary(), &[selection(&sketch)]).unwrap();
    let mut repeated = selection(&sketch);
    repeated
        .condition_candidate_ids
        .push(sketch.conditions[0].id.clone());
    repeated
        .package_variant_candidate_ids
        .push(sketch.packages[0].id.clone());
    let actual = assemble_claim_bundle(&sketch, &vocabulary(), &[repeated]).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn claim_bundle_rejects_missing_condition_and_evidence_references() {
    let sketch = sketch();
    let mut missing_condition = selection(&sketch);
    missing_condition.condition_candidate_ids = vec![CandidateId("condition:missing".into())];
    assert!(assemble_claim_bundle(&sketch, &vocabulary(), &[missing_condition]).is_err());

    let mut bundle = assemble_claim_bundle(&sketch, &vocabulary(), &[selection(&sketch)]).unwrap();
    bundle.claims[0].evidence_anchors[0].evidence_ids = vec!["e-missing".into()];
    assert!(validate_claim_bundle_against_sketch(&bundle, &sketch).is_err());
}

#[test]
fn claim_bundle_rejects_generated_value_tampering_and_stale_claim_identity() {
    let sketch = sketch();
    let mut generated =
        assemble_claim_bundle(&sketch, &vocabulary(), &[selection(&sketch)]).unwrap();
    generated.selected_values[0].raw_text = "9.99 V".into();
    generated.selected_values[0]
        .normalized_quantity
        .as_mut()
        .unwrap()
        .canonical_value = "9.99".into();
    assert!(validate_claim_bundle_against_sketch(&generated, &sketch).is_err());

    let mut stale_id =
        assemble_claim_bundle(&sketch, &vocabulary(), &[selection(&sketch)]).unwrap();
    stale_id.claims[0].predicate_term_id = "electrical.changed".into();
    assert!(validate_claim_bundle_against_sketch(&stale_id, &sketch).is_err());

    let mut generated_subject =
        assemble_claim_bundle(&sketch, &vocabulary(), &[selection(&sketch)]).unwrap();
    generated_subject.subject_hints[0].mpn_raw = "GENERATED-PART".into();
    assert!(validate_claim_bundle_against_sketch(&generated_subject, &sketch).is_err());

    let bundle = assemble_claim_bundle(&sketch, &vocabulary(), &[selection(&sketch)]).unwrap();
    let mut missing_predicate = vocabulary();
    missing_predicate.terms.clear();
    assert!(
        validate_claim_bundle_against_sketch_and_vocabulary(&bundle, &sketch, &missing_predicate)
            .is_err()
    );
}

#[test]
fn identical_table_shapes_in_distinct_regions_have_distinct_candidate_ids() {
    let mut fixture = fixture();
    let mut duplicate = fixture.pages[0]
        .regions
        .iter()
        .find(|region| region.table_shape.is_some())
        .unwrap()
        .clone();
    duplicate.region_id = "region-table-duplicate".into();
    for (index, evidence) in duplicate.evidence.iter_mut().enumerate() {
        evidence.evidence_id = format!("duplicate-cell-{index}");
    }
    fixture.pages[0].regions.push(duplicate);
    let sketch = sketch_from_fixture(&fixture, producer()).unwrap();
    assert_eq!(sketch.tables.len(), 2);
    assert_ne!(sketch.tables[0].id, sketch.tables[1].id);
}

#[test]
fn ontology_snapshot_participates_in_sketch_identity() {
    let first = sketch();
    let mut changed_producer = producer();
    changed_producer.ontology_snapshot_digest = Sha256Digest::of_bytes(b"different ontology");
    let changed = sketch_from_fixture(&fixture(), changed_producer).unwrap();
    assert_ne!(first.sketch_id, changed.sketch_id);
}

#[test]
fn document_schema_and_quantitas_registry_are_bound_fail_closed() {
    let mut wrong_schema = producer();
    wrong_schema.document_ir_schema = "different-document-ir/1".into();
    assert!(sketch_from_fixture(&fixture(), wrong_schema).is_err());

    let mut wrong_registry = sketch();
    wrong_registry.quantities[0]
        .normalized_quantity
        .as_mut()
        .unwrap()
        .registry_identity = "different-registry/1".into();
    wrong_registry.sketch_id = wrong_registry.recompute_id().unwrap();
    assert!(wrong_registry.validate().is_err());
}

#[test]
fn checked_in_schema_snapshots_are_current() {
    let cases = [
        (
            include_str!("../../../schemas/fdx1/evidence-anchor-v2.schema.json"),
            serde_json::to_value(schemars::schema_for!(EvidenceAnchor)).unwrap(),
        ),
        (
            include_str!("../../../schemas/fdx1/datasheet-sketch-v1.schema.json"),
            serde_json::to_value(schemars::schema_for!(DatasheetSketch)).unwrap(),
        ),
        (
            include_str!("../../../schemas/fdx1/claim-bundle-v1.schema.json"),
            serde_json::to_value(schemars::schema_for!(ClaimBundle)).unwrap(),
        ),
    ];
    for (snapshot, generated) in cases {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(snapshot).unwrap(),
            generated
        );
    }
}
