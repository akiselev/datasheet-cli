use std::{collections::BTreeMap, fs, path::Path};

use datasheet_compiler::supervision::{
    CombinationResult, LabelVoteInput, SourceVocabularyMapInput, build_label_vote, combine_votes,
    derive_source_vocabulary_map, freeze_calibration_snapshot, freeze_reviewed_development_set,
};
use datasheet_core::supervision::{
    LabelVote, LabelerCalibration, LabelerRef, LineageGroup, ReviewedDevelopmentExample,
    ReviewedDevelopmentSet, RightsSnapshot, SourceVocabularyMap, SupervisionObservation,
    SupervisionProducer, TRAINING_EXAMPLE_SCHEMA, TrainingExample, TrainingLabel, VoteOutcome,
};
use datasheet_core::{
    CandidateId, ParsedQuantityShape, PredicateTerm, PredicateVocabularySnapshot, Sha256Digest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const SOURCE_SHA: &str = "2cb2dff3c03952804b7b0abc0648289242c17cf34d829ad94ad2b796adeae284";
const FACT_EXPORT_SHA: &str = "4ebfd791db8972b99309600e3347cc77f24dc27e3c02962739ace73d1c7d7912";
const FOUNDRY_REVISION: &str = "d4e35dda5fa1f1b78347a4fec2e26b1b06c3e718";

#[derive(Clone, Deserialize)]
struct Fact {
    record_id: String,
    external_namespace: String,
    external_schema_version: String,
    external_field_key: String,
    raw_value: String,
    source_metadata_json: String,
}

#[derive(Deserialize)]
struct Review {
    source_markdown_sha256: String,
    foundry_export_sha256: String,
    rows: Vec<ReviewedRow>,
}

#[derive(Deserialize)]
struct ReviewedRow {
    record_id: String,
    external_field_key: String,
    raw_value: String,
    source_metadata: Value,
}

#[derive(Serialize)]
struct Bundle {
    schema_version: &'static str,
    source_artifact_sha256: &'static str,
    foundry_revision: &'static str,
    source_fact_export_sha256: &'static str,
    source_fact_count: usize,
    reviewed_fact_count: usize,
    vocabulary: PredicateVocabularySnapshot,
    vocabulary_maps: Vec<SourceVocabularyMap>,
    reviewed_development_set: ReviewedDevelopmentSet,
    calibration_snapshot: datasheet_core::supervision::CalibrationSnapshot,
    votes: Vec<LabelVote>,
    labels: Vec<TrainingLabel>,
    training_examples: Vec<TrainingExample>,
    limitation_codes: Vec<String>,
}

fn digest(value: &impl Serialize) -> Sha256Digest {
    Sha256Digest::of_bytes(&serde_json::to_vec(value).expect("serializable"))
}

fn producer() -> SupervisionProducer {
    SupervisionProducer {
        name: "datasheet-fdx4-tpx00242-reviewed-source-native".into(),
        version: "1".into(),
        build: Some(format!("foundry-{FOUNDRY_REVISION}")),
        configuration_digest: digest(&("tpx00242-reviewed-table-mapping", 1_u8)),
        model_digest: None,
    }
}

fn labeler(id: &str) -> LabelerRef {
    LabelerRef {
        labeler_id: id.into(),
        labeler_version: "1".into(),
    }
}

fn selected(key: &str) -> bool {
    [
        "INPUTSPECIFICATIONS.",
        "OUTPUTSPECIFICATIONS.",
        "VOLTAGEREGULATION.",
        "RIPPLEANDNOISE.",
        "DYNAMICRESPONSE.",
        "EFFICIENCY.",
        "OPERATINGCONDITIONS.",
        "ELECTRICALSAFETY.",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}

fn shape(raw: &str) -> ParsedQuantityShape {
    if raw.contains(" - ") || raw.contains('±') {
        ParsedQuantityShape::Range
    } else {
        ParsedQuantityShape::Scalar
    }
}

fn vocabulary(facts: &[Fact]) -> PredicateVocabularySnapshot {
    let mut by_key = BTreeMap::<String, Vec<ParsedQuantityShape>>::new();
    for fact in facts {
        by_key
            .entry(fact.external_field_key.clone())
            .or_default()
            .push(shape(&fact.raw_value));
    }
    let mut terms = by_key.into_iter().map(|(key, mut shapes)| {
        shapes.sort(); shapes.dedup();
        PredicateTerm {
            id: format!("power_supply.tpx00242.{}", key.to_ascii_lowercase()),
            canonical_name: key.replace('.', " / "),
            description: "Exact source-native TPX00242 table field; row conditions remain in the reviewed target metadata.".into(),
            aliases: vec![], symbol_aliases: vec![], expected_quantity_kind: None,
            expected_dimensionality: None, expected_shapes: shapes, regime_constraints: vec![],
            category_scope: vec!["analog_power".into(), "power_supply".into()],
        }
    }).collect::<Vec<_>>();
    terms.sort_by(|a, b| a.id.cmp(&b.id));
    PredicateVocabularySnapshot {
        schema_version: "predicate-vocabulary/1".into(),
        digest: digest(&("predicate-vocabulary/1", &terms)),
        terms,
    }
}

fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: build_fdx4_tpx00242_supervision <output>");
    let facts_bytes = include_bytes!("../../../evidence/fdx4-tpx00242-foundry-facts.json");
    assert_eq!(
        Sha256Digest::of_bytes(facts_bytes).0,
        FACT_EXPORT_SHA,
        "Foundry fact export drift"
    );
    let facts = serde_json::from_slice::<Vec<Fact>>(facts_bytes)
        .unwrap()
        .into_iter()
        .filter(|fact| selected(&fact.external_field_key))
        .collect::<Vec<_>>();
    assert_eq!(facts.len(), 55, "reviewed electrical table shape drift");
    assert!(
        facts
            .iter()
            .all(|fact| fact.external_namespace == "ARDUINO.DOCS.TPX00242"
                && fact.external_schema_version
                    == "docs-content/ad72210c6dffa698374d4f834aae2d568e49b41d")
    );
    let review: Review = serde_json::from_slice(include_bytes!(
        "../../../evidence/fdx4-tpx00242-independent-review.json"
    ))
    .unwrap();
    assert_eq!(review.source_markdown_sha256, SOURCE_SHA);
    assert_eq!(review.foundry_export_sha256, FACT_EXPORT_SHA);
    assert_eq!(review.rows.len(), facts.len());
    let reviewed = review
        .rows
        .iter()
        .map(|row| (&row.record_id, row))
        .collect::<BTreeMap<_, _>>();
    for fact in &facts {
        let row = reviewed
            .get(&fact.record_id)
            .expect("every admitted fact independently reviewed");
        assert_eq!(row.external_field_key, fact.external_field_key);
        assert_eq!(row.raw_value, fact.raw_value);
        assert_eq!(
            row.source_metadata,
            serde_json::from_str::<Value>(&fact.source_metadata_json).unwrap()
        );
    }

    let vocabulary = vocabulary(&facts);
    let lineage = {
        let mut value = LineageGroup {
            group_id: digest(&"placeholder"),
            publisher_key: Some("publisher.arduino".into()),
            upstream_dataset_fingerprint: Some(Sha256Digest(SOURCE_SHA.into())),
            source_schema: Some("ARDUINO.DOCS.TPX00242/docs-content-ad72210".into()),
            observation_epoch: Some("2026-08-17".into()),
        };
        value.group_id = value.recompute_id();
        value
    };
    let rights = RightsSnapshot {
        snapshot_id: digest(&("policy.arduino.docs.cc-by-sa", "2026-08-22", SOURCE_SHA)),
        policy_id: "policy.arduino.docs.cc-by-sa".into(),
        policy_version: "2026-08-22".into(),
        retention_permitted: true,
        training_permitted: true,
        remote_disclosure_permitted: false,
    };
    let observations = facts
        .iter()
        .map(|fact| {
            let source_record_id = Sha256Digest(fact.record_id.clone());
            let observation = SupervisionObservation {
                observation_id: digest(&(SOURCE_SHA, &fact.record_id, &fact.raw_value)),
                source_record_id: source_record_id.clone(),
                source_schema: "bronze.structured_fact_observations/v1".into(),
                producer: producer(),
                document_state: None,
                candidate_ids: vec![],
                evidence_anchors: vec![],
                lineage_groups: vec![lineage.clone()],
                rights_snapshots: vec![rights.clone()],
            };
            (source_record_id, observation)
        })
        .collect::<BTreeMap<_, _>>();

    let mut maps = Vec::new();
    for term in &vocabulary.terms {
        let key = term
            .id
            .strip_prefix("power_supply.tpx00242.")
            .unwrap()
            .to_ascii_uppercase();
        let ids = facts
            .iter()
            .filter(|fact| fact.external_field_key == key)
            .map(|fact| {
                observations[&Sha256Digest(fact.record_id.clone())]
                    .observation_id
                    .clone()
            })
            .collect();
        maps.push(
            derive_source_vocabulary_map(
                SourceVocabularyMapInput {
                    source_schema: "bronze.structured_fact_observations/v1".into(),
                    external_namespace: "ARDUINO.DOCS.TPX00242".into(),
                    external_field_key: key,
                    predicate_term_id: term.id.clone(),
                    expected_quantity_kind: None,
                    expected_value_shapes: term.expected_shapes.clone(),
                    regime_hint: None,
                    category_scope: term.category_scope.clone(),
                    mapping_observation_ids: ids,
                    producer: producer(),
                    mapping_version: "docs-content-ad72210-reviewed/1".into(),
                },
                &vocabulary,
            )
            .unwrap(),
        );
    }
    let map_by_key = maps
        .iter()
        .map(|map| (map.external_field_key.as_str(), map))
        .collect::<BTreeMap<_, _>>();
    let targets = facts.iter().map(|fact| {
        let metadata: Value = serde_json::from_str(&fact.source_metadata_json).unwrap();
        json!({"external_field_key":fact.external_field_key,"predicate_term_id":map_by_key[fact.external_field_key.as_str()].predicate_term_id,
            "source_part_number":"TPX00242","raw_value":fact.raw_value,"source_table":metadata["table"],
            "source_row_key":metadata["row_key"],"source_line":metadata["source_line"],"regime":null})
    }).collect::<Vec<_>>();
    let development_examples = facts
        .iter()
        .zip(&targets)
        .map(|(fact, target)| {
            let source_id = Sha256Digest(fact.record_id.clone());
            ReviewedDevelopmentExample {
                example_id: digest(&("reviewed-tpx00242", &source_id, target)),
                task: "structured_fact_semantics".into(),
                training_candidate_id: CandidateId(format!("arduino-tpx00242:{}", fact.record_id)),
                expected_target_json: target.clone(),
                observation_ids: vec![observations[&source_id].observation_id.clone()],
                reviewer_record_id: digest(&(
                    "independent-primary-source-table-review",
                    SOURCE_SHA,
                    &fact.record_id,
                    target,
                )),
            }
        })
        .collect();
    let development = freeze_reviewed_development_set(
        vocabulary.digest.clone(),
        development_examples,
        producer(),
    )
    .unwrap();
    let scalar = facts
        .iter()
        .filter(|fact| shape(&fact.raw_value) == ParsedQuantityShape::Scalar)
        .count() as u32;
    let range = u32::try_from(facts.len()).unwrap() - scalar;
    let calibration = freeze_calibration_snapshot(
        development.digest.clone(),
        vocabulary.digest.clone(),
        vec![
            LabelerCalibration {
                labeler: labeler("source.vocabulary_map.v1"),
                reviewed_correct: 55,
                reviewed_total: 55,
                precision_millionths: 1_000_000,
                weight_millionths: 1_000_000,
            },
            LabelerCalibration {
                labeler: labeler("source.value_align_unique.v1"),
                reviewed_correct: scalar,
                reviewed_total: scalar,
                precision_millionths: 1_000_000,
                weight_millionths: 1_000_000,
            },
            LabelerCalibration {
                labeler: labeler("source.range_align.v1"),
                reviewed_correct: range,
                reviewed_total: range,
                precision_millionths: 1_000_000,
                weight_millionths: 1_000_000,
            },
        ],
        1_000_000,
        1,
        producer(),
    )
    .unwrap();

    let mut votes = Vec::new();
    let mut labels = Vec::new();
    let mut examples = Vec::new();
    for (fact, target) in facts.iter().zip(&targets) {
        let source_id = Sha256Digest(fact.record_id.clone());
        let observation = observations[&source_id].clone();
        let candidate = CandidateId(format!("arduino-tpx00242:{}", fact.record_id));
        let fact_shape = shape(&fact.raw_value);
        let mut fact_votes = Vec::new();
        for id in [
            "source.vocabulary_map.v1",
            "source.value_align_unique.v1",
            "source.range_align.v1",
        ] {
            let applicable = id == "source.vocabulary_map.v1"
                || (id == "source.range_align.v1") == (fact_shape == ParsedQuantityShape::Range);
            let outcome = if applicable {
                VoteOutcome::Vote {
                    vote_json: target.clone(),
                }
            } else {
                VoteOutcome::Abstain {
                    abstain_reason: "labeler_not_applicable_to_reviewed_cell_shape".into(),
                }
            };
            let vote = build_label_vote(LabelVoteInput {
                training_candidate_id: candidate.clone(),
                task: "structured_fact_semantics".into(),
                labeler: labeler(id),
                outcome,
                observations: vec![observation.clone()],
                reason_codes: vec![if applicable {
                    "exact_reviewed_source_cell".into()
                } else {
                    "shape_mismatch_abstention".into()
                }],
                producer: producer(),
            })
            .unwrap();
            fact_votes.push(vote.clone());
            votes.push(vote);
        }
        let CombinationResult::Label { label } = combine_votes(
            &candidate,
            "structured_fact_semantics",
            &fact_votes,
            &calibration,
        )
        .unwrap() else {
            panic!("reviewed fact abstained")
        };
        let example = TrainingExample {
            schema_version: TRAINING_EXAMPLE_SCHEMA.into(),
            example_id: digest(&(
                "tpx00242-training-example",
                &label.training_label_id,
                &source_id,
            )),
            task: "structured_fact_semantics".into(),
            document_state_id: None,
            document_ir_logical_sha256: None,
            datasheet_sketch_digest: None,
            observation_ids: vec![observation.observation_id],
            source_record_ids: vec![source_id],
            candidate_ids: vec![candidate],
            training_label_record_id: label.training_label_id.clone(),
            document_family_id: "manufacturer.arduino.tpx00242.docs-content".into(),
            template_family_id: Some("arduino.docs-content.accessory-datasheet".into()),
            manufacturer_key: Some("arduino".into()),
            assigned_split: "development".into(),
            ontology_snapshot_digest: vocabulary.digest.clone(),
            rights_snapshot_ids: vec![rights.snapshot_id.clone()],
            producer: producer(),
        };
        example.validate_structure().unwrap();
        labels.push(label);
        examples.push(example);
    }
    let bundle = Bundle {
        schema_version: "fdx4-tpx00242-supervision-bundle/1",
        source_artifact_sha256: SOURCE_SHA,
        foundry_revision: FOUNDRY_REVISION,
        source_fact_export_sha256: FACT_EXPORT_SHA,
        source_fact_count: facts.len(),
        reviewed_fact_count: review.rows.len(),
        vocabulary,
        vocabulary_maps: maps,
        reviewed_development_set: development,
        calibration_snapshot: calibration,
        votes,
        labels,
        training_examples: examples,
        limitation_codes: vec![
            "single_arduino_document_revision".into(),
            "source_native_cells_not_normalized_regimes".into(),
            "not_final_blind".into(),
        ],
    };
    let mut bytes = serde_json::to_vec_pretty(&bundle).unwrap();
    bytes.push(b'\n');
    fs::write(Path::new(&output), bytes).unwrap();
}
