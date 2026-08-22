use datasheet_compiler::supervision::{
    CombinationAbstentionReason, CombinationResult, LABELER_CATALOG, LabelVoteInput,
    SourceVocabularyMapInput, build_label_vote, combine_votes, derive_source_vocabulary_map,
    freeze_calibration_snapshot, freeze_reviewed_development_set, frozen_labeler_catalog,
    validate_source_vocabulary_map, validate_training_label,
};
use datasheet_core::supervision::{
    CalibrationSnapshot, LabelVote, LabelerCalibration, LabelerRef, ReviewedDevelopmentExample,
    ReviewedDevelopmentSet, SourceVocabularyMap, StructuredFactRecord, SupervisionObservation,
    SupervisionProducer, TrainingExample, TrainingLabel, VoteOutcome,
};
use datasheet_core::{CandidateId, PredicateVocabularySnapshot, Sha256Digest};
use serde::Deserialize;
use serde_json::json;

const FIXTURE: &str = include_str!("../../../tests/fixtures/fdx3_synthetic_fls5_supervision.json");
const VOCABULARY: &str = include_str!("../../../tests/fixtures/fdx2_predicate_vocabulary.json");

#[derive(Clone, Deserialize)]
struct Fixture {
    document_observation: SupervisionObservation,
    structured_observations: Vec<SupervisionObservation>,
}

fn fixture() -> Fixture {
    serde_json::from_str(FIXTURE).unwrap()
}

fn digest(byte: char) -> Sha256Digest {
    Sha256Digest(std::iter::repeat_n(byte, 64).collect())
}

fn producer() -> SupervisionProducer {
    SupervisionProducer {
        name: "datasheet-fdx3-synthetic".into(),
        version: "1".into(),
        build: Some("test".into()),
        configuration_digest: digest('7'),
        model_digest: None,
    }
}

fn labeler(id: &str) -> LabelerRef {
    LabelerRef {
        labeler_id: id.into(),
        labeler_version: "1".into(),
    }
}

fn vote(
    id: &str,
    outcome: VoteOutcome,
    observations: Vec<SupervisionObservation>,
) -> datasheet_core::supervision::LabelVote {
    build_label_vote(LabelVoteInput {
        training_candidate_id: CandidateId("training:vin-max".into()),
        task: "predicate_linking".into(),
        labeler: labeler(id),
        outcome,
        observations,
        reason_codes: vec!["synthetic_fixture_evidence".into()],
        producer: producer(),
    })
    .unwrap()
}

fn calibration(
    ids: &[&str],
    minimum_lineages: u32,
) -> datasheet_core::supervision::CalibrationSnapshot {
    freeze_calibration_snapshot(
        digest('8'),
        digest('3'),
        ids.iter()
            .map(|id| LabelerCalibration {
                labeler: labeler(id),
                reviewed_correct: 9,
                reviewed_total: 10,
                precision_millionths: 900_000,
                weight_millionths: 900_000,
            })
            .collect(),
        600_000,
        minimum_lineages,
        producer(),
    )
    .unwrap()
}

#[test]
fn catalog_contains_every_plan_identity_once() {
    let expected = [
        "section.heading.v1",
        "section.table_header_schema.v1",
        "cell.min_typ_max_header.v1",
        "quantity.dimension.v1",
        "predicate.symbol_alias.v1",
        "predicate.row_name_alias.v1",
        "alignment.value_unique.v1",
        "alignment.range.v1",
        "template.cluster.v1",
        "applicability.footnote_link.v1",
        "scope.package.v1",
        "scope.part.v1",
        "value.null_dash_convention.v1",
        "ranking.regime_conflict.v1",
        "revision.structure_stability.v1",
        "ti.identity_gate.v1",
        "ti.field_map.v1",
        "ti.value_align_unique.v1",
        "ti.range_align.v1",
        "ti.unit_kind.v1",
        "ti.package_align.v1",
        "digikey.parameter_id_map.v1",
        "digikey.parameter_text_alias.v1",
        "digikey.value_align_unique.v1",
        "digikey.category_prior.v1",
        "digikey.package_scope.v1",
        "digikey.datasheet_link.v1",
        "mouser.mpn_identity.v1",
        "mouser.datasheet_link.v1",
        "mouser.category.v1",
        "mouser.package.v1",
        "mouser.lifecycle.v1",
        "mouser.replacement.v1",
        "manufacturer.field_map.v1",
        "manufacturer.value_align.v1",
        "manufacturer.range_align.v1",
        "manufacturer.package.v1",
        "manufacturer.variant_scope.v1",
        "manufacturer.order_code.v1",
        "manufacturer.family_scope.v1",
        "eda.pin_number_set.v1",
        "eda.pin_name_by_number.v1",
        "eda.pin_count.v1",
        "eda.package_code.v1",
        "eda.pad_count.v1",
        "eda.pin_to_pad.v1",
        "eda.body_dimensions.v1",
        "eda.model_package.v1",
        "cmsis.device_identity.v1",
        "cmsis.cpu_core.v1",
        "cmsis.memory.v1",
        "cmsis.document_link.v1",
        "svd.peripheral.v1",
        "svd.register.v1",
    ];
    let actual = LABELER_CATALOG
        .iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    for required in expected {
        assert!(actual.contains(&required), "missing {required}");
    }
    let unique = actual.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), actual.len());

    let first = frozen_labeler_catalog().unwrap();
    let second = frozen_labeler_catalog().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.labelers.len(), LABELER_CATALOG.len());
    assert_eq!(first.combiners.len(), 4);
    assert_eq!(
        format!("{}\n", serde_json::to_string_pretty(&first).unwrap()),
        include_str!("../../../tests/goldens/fdx3_labeler_catalog.json")
    );
}

#[test]
fn reviewed_development_and_calibration_are_frozen_order_independently() {
    let example = |byte, observation_bytes: [char; 2]| ReviewedDevelopmentExample {
        example_id: digest(byte),
        task: "predicate_linking".into(),
        training_candidate_id: CandidateId(format!("training:{byte}")),
        expected_target_json: json!("analog.input_voltage.absolute_max"),
        observation_ids: observation_bytes.into_iter().map(digest).collect(),
        reviewer_record_id: digest('9'),
    };
    let first = freeze_reviewed_development_set(
        digest('3'),
        vec![example('2', ['5', '4']), example('1', ['7', '6'])],
        producer(),
    )
    .unwrap();
    let second = freeze_reviewed_development_set(
        digest('3'),
        vec![example('1', ['6', '7']), example('2', ['4', '5'])],
        producer(),
    )
    .unwrap();
    assert_eq!(first, second);
    let calibration = freeze_calibration_snapshot(
        first.digest.clone(),
        digest('3'),
        vec![LabelerCalibration {
            labeler: labeler("predicate.row_name_alias.v1"),
            reviewed_correct: 2,
            reviewed_total: 2,
            precision_millionths: 1_000_000,
            weight_millionths: 1_000_000,
        }],
        900_000,
        1,
        producer(),
    )
    .unwrap();
    assert_eq!(calibration.reviewed_development_set_digest, first.digest);
}

#[test]
fn vocabulary_mapping_is_order_independent_and_ontology_bound() {
    let vocabulary: PredicateVocabularySnapshot = serde_json::from_str(VOCABULARY).unwrap();
    let input = SourceVocabularyMapInput {
        source_schema: "synthetic.pim/v1".into(),
        external_namespace: "synthetic".into(),
        external_field_key: "VIN_MAX".into(),
        predicate_term_id: "analog.input_voltage.absolute_max".into(),
        expected_quantity_kind: Some("electronics:ElectricPotential".into()),
        expected_value_shapes: vec![
            datasheet_core::ParsedQuantityShape::Symbolic,
            datasheet_core::ParsedQuantityShape::Scalar,
        ],
        regime_hint: Some("absolute_maximum".into()),
        category_scope: vec!["power".into(), "analog_power".into()],
        mapping_observation_ids: vec![digest('2'), digest('1')],
        producer: producer(),
        mapping_version: "1".into(),
    };
    let first = derive_source_vocabulary_map(input.clone(), &vocabulary).unwrap();
    let mut reordered = input;
    reordered.mapping_observation_ids.reverse();
    reordered.expected_value_shapes.reverse();
    reordered.category_scope.reverse();
    let second = derive_source_vocabulary_map(reordered, &vocabulary).unwrap();
    assert_eq!(first, second);
    validate_source_vocabulary_map(&first, &vocabulary).unwrap();
    let mut tampered = first.clone();
    tampered.external_field_key = "TAMPERED".into();
    assert!(validate_source_vocabulary_map(&tampered, &vocabulary).is_err());

    let mut missing = vocabulary;
    missing.terms.clear();
    assert!(
        derive_source_vocabulary_map(
            SourceVocabularyMapInput {
                source_schema: first.source_schema,
                external_namespace: first.external_namespace,
                external_field_key: first.external_field_key,
                predicate_term_id: first.predicate_term_id,
                expected_quantity_kind: first.expected_quantity_kind,
                expected_value_shapes: first.expected_value_shapes,
                regime_hint: first.regime_hint,
                category_scope: first.category_scope,
                mapping_observation_ids: first.mapping_observation_ids,
                producer: first.producer,
                mapping_version: first.mapping_version,
            },
            &missing
        )
        .is_err()
    );
}

#[test]
fn observations_require_exact_pinned_state_and_votes_are_order_independent() {
    let fixture = fixture();
    fixture.document_observation.validate().unwrap();
    let mut stale = fixture.document_observation.clone();
    stale.document_state.as_mut().unwrap().document_state_id = "state_stale".into();
    assert!(stale.validate().is_err());
    let mut no_state = fixture.document_observation.clone();
    no_state.document_state = None;
    assert!(no_state.validate().is_err());
    let mut false_lineage = fixture.structured_observations[0].clone();
    false_lineage.lineage_groups[0].publisher_key = Some("tampered-copy".into());
    assert!(false_lineage.validate().is_err());

    let first = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote {
            vote_json: json!("analog.input_voltage.absolute_max"),
        },
        fixture.structured_observations[..2].to_vec(),
    );
    let mut reversed = fixture.structured_observations[..2].to_vec();
    reversed.reverse();
    let second = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote {
            vote_json: json!("analog.input_voltage.absolute_max"),
        },
        reversed,
    );
    assert_eq!(first, second);

    let abstention = vote(
        "predicate.symbol_alias.v1",
        VoteOutcome::Abstain {
            abstain_reason: "symbol_not_found".into(),
        },
        vec![fixture.document_observation],
    );
    assert!(matches!(abstention.outcome, VoteOutcome::Abstain { .. }));
    assert!(!abstention.reason_codes.is_empty());
}

#[test]
fn checked_in_fdx3_schemas_match_rust_contracts() {
    fn check<T: schemars::JsonSchema>(bytes: &str) {
        let expected: serde_json::Value = serde_json::from_str(bytes).unwrap();
        assert_eq!(
            serde_json::to_value(schemars::schema_for!(T)).unwrap(),
            expected
        );
    }
    check::<SourceVocabularyMap>(include_str!(
        "../../../schemas/fdx3/source-vocabulary-map-v1.schema.json"
    ));
    check::<datasheet_compiler::supervision::LabelerCatalogArtifact>(include_str!(
        "../../../schemas/fdx3/labeler-catalog-v1.schema.json"
    ));
    check::<LabelVote>(include_str!(
        "../../../schemas/fdx3/silver-label-votes-v1.schema.json"
    ));
    check::<TrainingLabel>(include_str!(
        "../../../schemas/fdx3/silver-training-labels-v1.schema.json"
    ));
    check::<StructuredFactRecord>(include_str!(
        "../../../schemas/fdx3/silver-structured-facts-v1.schema.json"
    ));
    check::<TrainingExample>(include_str!(
        "../../../schemas/fdx3/training-examples-v1.schema.json"
    ));
    check::<ReviewedDevelopmentSet>(include_str!(
        "../../../schemas/fdx3/reviewed-development-set-v1.schema.json"
    ));
    check::<CalibrationSnapshot>(include_str!(
        "../../../schemas/fdx3/labeler-calibration-v1.schema.json"
    ));
    check::<CombinationResult>(include_str!(
        "../../../schemas/fdx3/combination-result-v1.schema.json"
    ));
}

#[test]
fn combiner_deduplicates_lineage_and_retains_abstentions_as_inputs() {
    let fixture = fixture();
    let target = json!("analog.input_voltage.absolute_max");
    let copied_a = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote {
            vote_json: target.clone(),
        },
        vec![fixture.structured_observations[0].clone()],
    );
    let copied_b = vote(
        "manufacturer.field_map.v1",
        VoteOutcome::Vote {
            vote_json: target.clone(),
        },
        vec![fixture.structured_observations[1].clone()],
    );
    let independent = vote(
        "quantity.dimension.v1",
        VoteOutcome::Vote {
            vote_json: target.clone(),
        },
        vec![fixture.structured_observations[2].clone()],
    );
    let absent = vote(
        "predicate.symbol_alias.v1",
        VoteOutcome::Abstain {
            abstain_reason: "symbol_not_found".into(),
        },
        vec![fixture.document_observation],
    );
    let calibration = calibration(
        &[
            "predicate.row_name_alias.v1",
            "manufacturer.field_map.v1",
            "quantity.dimension.v1",
        ],
        2,
    );
    let result = combine_votes(
        &CandidateId("training:vin-max".into()),
        "predicate_linking",
        &[copied_b, absent, independent, copied_a],
        &calibration,
    )
    .unwrap();
    let CombinationResult::Label { label } = result else {
        panic!("expected label")
    };
    validate_training_label(&label).unwrap();
    assert_eq!(label.target_json, target);
    assert_eq!(label.uncertainty.independent_lineage_count, 2);
    assert_eq!(label.input_vote_record_ids.len(), 4);
    let mut tampered = label;
    tampered.target_json = json!("ungrounded-tamper");
    assert!(validate_training_label(&tampered).is_err());
}

#[test]
fn duplicate_lineage_cannot_fake_independent_support() {
    let fixture = fixture();
    let target = json!("analog.input_voltage.absolute_max");
    let a = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote {
            vote_json: target.clone(),
        },
        vec![fixture.structured_observations[0].clone()],
    );
    let b = vote(
        "manufacturer.field_map.v1",
        VoteOutcome::Vote { vote_json: target },
        vec![fixture.structured_observations[1].clone()],
    );
    let result = combine_votes(
        &CandidateId("training:vin-max".into()),
        "predicate_linking",
        &[a, b],
        &calibration(
            &["predicate.row_name_alias.v1", "manufacturer.field_map.v1"],
            2,
        ),
    )
    .unwrap();
    let CombinationResult::Abstain { abstention } = result else {
        panic!("expected abstention")
    };
    assert_eq!(
        abstention.reason,
        CombinationAbstentionReason::InsufficientIndependentLineage
    );
}

#[test]
fn rights_conflict_and_failed_identity_gate_abstain_instead_of_negative_labels() {
    let fixture = fixture();
    let target = json!("analog.input_voltage.absolute_max");
    let mut forbidden = fixture.structured_observations[0].clone();
    forbidden.rights_snapshots[0].training_permitted = false;
    let value = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote { vote_json: target },
        vec![forbidden],
    );
    let result = combine_votes(
        &CandidateId("training:vin-max".into()),
        "predicate_linking",
        &[value],
        &calibration(&["predicate.row_name_alias.v1"], 1),
    )
    .unwrap();
    assert!(
        matches!(result, CombinationResult::Abstain { abstention } if abstention.reason == CombinationAbstentionReason::RightsForbidden)
    );

    let gate = vote(
        "ti.identity_gate.v1",
        VoteOutcome::Abstain {
            abstain_reason: "subject_ambiguous".into(),
        },
        vec![fixture.structured_observations[1].clone()],
    );
    let value = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote {
            vote_json: json!("not-a-negative-label"),
        },
        vec![fixture.structured_observations[1].clone()],
    );
    let result = combine_votes(
        &CandidateId("training:vin-max".into()),
        "predicate_linking",
        &[value, gate],
        &calibration(&["predicate.row_name_alias.v1"], 1),
    )
    .unwrap();
    assert!(
        matches!(result, CombinationResult::Abstain { abstention } if abstention.reason == CombinationAbstentionReason::HardIdentityGate)
    );
}

#[test]
fn source_conflict_abstains_without_fabricating_a_winner() {
    let fixture = fixture();
    let left = vote(
        "predicate.row_name_alias.v1",
        VoteOutcome::Vote {
            vote_json: json!("left"),
        },
        vec![fixture.structured_observations[0].clone()],
    );
    let right = vote(
        "manufacturer.field_map.v1",
        VoteOutcome::Vote {
            vote_json: json!("right"),
        },
        vec![fixture.structured_observations[1].clone()],
    );
    let calibration = calibration(
        &["predicate.row_name_alias.v1", "manufacturer.field_map.v1"],
        1,
    );
    let result = combine_votes(
        &CandidateId("training:vin-max".into()),
        "predicate_linking",
        &[left, right],
        &calibration,
    )
    .unwrap();
    let CombinationResult::Abstain { abstention } = result else {
        panic!("expected source-conflict abstention")
    };
    assert_eq!(
        abstention.reason,
        CombinationAbstentionReason::SourceConflict
    );
    assert_eq!(abstention.combiner_id, "conflict.source.v1");
    assert_eq!(abstention.combiner_configuration_digest, calibration.digest);
}
