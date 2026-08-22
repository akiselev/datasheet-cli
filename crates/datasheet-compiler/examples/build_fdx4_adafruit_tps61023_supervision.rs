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

const SOURCE_SHA: &str = "67f6d35e78f275f2191feff064096a47862ab3f0b6bfd21f01071bbbcac4708d";
const LICENSE_SHA: &str = "075dad5e5fc96c27014fabc269f4f5732909cffd178a486f546d982b6cf86b74";
const FACT_EXPORT_SHA: &str = "d2636fd202ee73b70e3a51c683c5902519158ebdeda6ad9e08e4e0b8b154a305";
const REVIEW_SHA: &str = "1e0296ad487029b50ff263fa908253ee98a995cb3fe5d21c2fe777b22d85fa72";
const FOUNDRY_REVISION: &str = "61736730f6c7196538951f874b788d70e317a046";

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
    source_readme_sha256: String,
    source_license_sha256: String,
    source_commit: String,
    rows: Vec<ReviewedRow>,
}

#[derive(Deserialize)]
struct ReviewedRow {
    record_id: String,
    predicate: String,
    value: String,
    conditions: BTreeMap<String, String>,
    source_line: String,
}

#[derive(Serialize)]
struct Bundle {
    schema_version: &'static str,
    source_artifact_sha256: &'static str,
    source_license_sha256: &'static str,
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
        name: "datasheet-fdx4-adafruit-tps61023-reviewed-source-native".into(),
        version: "1".into(),
        build: Some(format!("foundry-{FOUNDRY_REVISION}")),
        configuration_digest: digest(&("adafruit-tps61023-reviewed-measurements", 1_u8)),
        model_digest: None,
    }
}

fn labeler(id: &str) -> LabelerRef {
    LabelerRef {
        labeler_id: id.into(),
        labeler_version: "1".into(),
    }
}

fn vocabulary() -> PredicateVocabularySnapshot {
    let terms = vec![
        PredicateTerm {
            id: "power_converter.measured_efficiency".into(),
            canonical_name: "Measured conversion efficiency".into(),
            description: "Board-publisher bench efficiency with all stated input/output conditions retained.".into(),
            aliases: vec![], symbol_aliases: vec![], expected_quantity_kind: None,
            expected_dimensionality: None, expected_shapes: vec![ParsedQuantityShape::Scalar],
            regime_constraints: vec![], category_scope: vec!["analog_power".into(), "power_converter".into()],
        },
        PredicateTerm {
            id: "power_converter.measured_maximum_output_current".into(),
            canonical_name: "Measured maximum output current".into(),
            description: "Board-publisher bench maximum current with input and output voltage conditions retained.".into(),
            aliases: vec![], symbol_aliases: vec![], expected_quantity_kind: None,
            expected_dimensionality: None, expected_shapes: vec![ParsedQuantityShape::Scalar],
            regime_constraints: vec![], category_scope: vec!["analog_power".into(), "power_converter".into()],
        },
    ];
    PredicateVocabularySnapshot {
        schema_version: "predicate-vocabulary/1".into(),
        digest: digest(&("predicate-vocabulary/1", &terms)),
        terms,
    }
}

fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: build_fdx4_adafruit_tps61023_supervision <output>");
    let fact_bytes = include_bytes!("../../../evidence/fdx4-adafruit-tps61023-foundry-facts.json");
    let review_bytes =
        include_bytes!("../../../evidence/fdx4-adafruit-tps61023-independent-review.json");
    assert_eq!(
        Sha256Digest::of_bytes(fact_bytes).0,
        FACT_EXPORT_SHA,
        "Foundry fact export drift"
    );
    assert_eq!(
        Sha256Digest::of_bytes(review_bytes).0,
        REVIEW_SHA,
        "independent review drift"
    );
    let facts: Vec<Fact> = serde_json::from_slice(fact_bytes).unwrap();
    let review: Review = serde_json::from_slice(review_bytes).unwrap();
    assert_eq!(facts.len(), 9);
    assert_eq!(review.rows.len(), 9);
    assert_eq!(review.source_readme_sha256, SOURCE_SHA);
    assert_eq!(review.source_license_sha256, LICENSE_SHA);
    assert_eq!(
        review.source_commit,
        "82b5a33a1900a5c13849029bc84e7856a44086e0"
    );
    assert!(
        facts
            .iter()
            .all(|fact| fact.external_namespace == "ADAFRUIT.DOCS.TPS61023"
                && fact.external_schema_version
                    == "github/82b5a33a1900a5c13849029bc84e7856a44086e0")
    );
    let reviewed = review
        .rows
        .iter()
        .map(|row| (&row.record_id, row))
        .collect::<BTreeMap<_, _>>();
    for fact in &facts {
        let row = reviewed
            .get(&fact.record_id)
            .expect("every source row independently reviewed");
        let metadata: Value = serde_json::from_str(&fact.source_metadata_json).unwrap();
        assert_eq!(row.value, fact.raw_value);
        assert_eq!(row.source_line, metadata["source_line"]);
        for (key, value) in &row.conditions {
            assert_eq!(Some(value.as_str()), metadata[key].as_str());
        }
        assert!(row.predicate.starts_with("power_converter.measured_"));
    }

    let vocabulary = vocabulary();
    let lineage = {
        let mut value = LineageGroup {
            group_id: digest(&"placeholder"),
            publisher_key: Some("publisher.adafruit".into()),
            upstream_dataset_fingerprint: Some(Sha256Digest(SOURCE_SHA.into())),
            source_schema: Some("ADAFRUIT.DOCS.TPS61023/github-82b5a33".into()),
            observation_epoch: Some("2026-08-22".into()),
        };
        value.group_id = value.recompute_id();
        value
    };
    let rights = RightsSnapshot {
        snapshot_id: digest(&(
            "policy.adafruit.tps61023.cc-by-sa-3.0",
            "2026-08-22",
            SOURCE_SHA,
            LICENSE_SHA,
        )),
        policy_id: "policy.adafruit.tps61023.cc-by-sa-3.0".into(),
        policy_version: "2026-08-22".into(),
        retention_permitted: true,
        training_permitted: true,
        remote_disclosure_permitted: false,
    };
    let observations = facts
        .iter()
        .map(|fact| {
            let source = Sha256Digest(fact.record_id.clone());
            let observation = SupervisionObservation {
                observation_id: digest(&(
                    SOURCE_SHA,
                    &fact.record_id,
                    &fact.raw_value,
                    &fact.source_metadata_json,
                )),
                source_record_id: source.clone(),
                source_schema: "bronze.structured_fact_observations/v1".into(),
                producer: producer(),
                document_state: None,
                candidate_ids: vec![],
                evidence_anchors: vec![],
                lineage_groups: vec![lineage.clone()],
                rights_snapshots: vec![rights.clone()],
            };
            (source, observation)
        })
        .collect::<BTreeMap<_, _>>();
    let mut maps = Vec::new();
    for (key, predicate) in [
        ("MEASURED.EFFICIENCY", "power_converter.measured_efficiency"),
        (
            "MEASURED.MAX_OUTPUT_CURRENT",
            "power_converter.measured_maximum_output_current",
        ),
    ] {
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
                    external_namespace: "ADAFRUIT.DOCS.TPS61023".into(),
                    external_field_key: key.into(),
                    predicate_term_id: predicate.into(),
                    expected_quantity_kind: None,
                    expected_value_shapes: vec![ParsedQuantityShape::Scalar],
                    regime_hint: None,
                    category_scope: vec!["analog_power".into(), "power_converter".into()],
                    mapping_observation_ids: ids,
                    producer: producer(),
                    mapping_version: "github-82b5a33-reviewed/1".into(),
                },
                &vocabulary,
            )
            .unwrap(),
        );
    }
    let map_by_key = maps
        .iter()
        .map(|map| {
            (
                map.external_field_key.as_str(),
                map.predicate_term_id.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let targets = facts.iter().map(|fact| {
        let row = reviewed[&fact.record_id];
        json!({"external_field_key":fact.external_field_key,"predicate_term_id":map_by_key[fact.external_field_key.as_str()],
            "source_part_number":"Adafruit 4654","raw_value":fact.raw_value,"conditions":row.conditions,
            "source_line":row.source_line,"regime":null})
    }).collect::<Vec<_>>();
    let development_examples = facts
        .iter()
        .zip(&targets)
        .map(|(fact, target)| {
            let source = Sha256Digest(fact.record_id.clone());
            ReviewedDevelopmentExample {
                example_id: digest(&("reviewed-adafruit-tps61023", &source, target)),
                task: "structured_fact_semantics".into(),
                training_candidate_id: CandidateId(format!("adafruit-tps61023:{}", fact.record_id)),
                expected_target_json: target.clone(),
                observation_ids: vec![observations[&source].observation_id.clone()],
                reviewer_record_id: digest(&(
                    "independent-primary-source-line-review",
                    REVIEW_SHA,
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
    let calibration = freeze_calibration_snapshot(
        development.digest.clone(),
        vocabulary.digest.clone(),
        vec![
            LabelerCalibration {
                labeler: labeler("source.vocabulary_map.v1"),
                reviewed_correct: 9,
                reviewed_total: 9,
                precision_millionths: 1_000_000,
                weight_millionths: 1_000_000,
            },
            LabelerCalibration {
                labeler: labeler("source.value_align_unique.v1"),
                reviewed_correct: 9,
                reviewed_total: 9,
                precision_millionths: 1_000_000,
                weight_millionths: 1_000_000,
            },
            LabelerCalibration {
                labeler: labeler("source.range_align.v1"),
                reviewed_correct: 9,
                reviewed_total: 9,
                precision_millionths: 1_000_000,
                weight_millionths: 1,
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
        let source = Sha256Digest(fact.record_id.clone());
        let observation = observations[&source].clone();
        let candidate = CandidateId(format!("adafruit-tps61023:{}", fact.record_id));
        let mut fact_votes = Vec::new();
        for id in [
            "source.vocabulary_map.v1",
            "source.value_align_unique.v1",
            "source.range_align.v1",
        ] {
            let applicable = id != "source.range_align.v1";
            let outcome = if applicable {
                VoteOutcome::Vote {
                    vote_json: target.clone(),
                }
            } else {
                VoteOutcome::Abstain {
                    abstain_reason: "labeler_not_applicable_to_reviewed_scalar_measurement".into(),
                }
            };
            let vote = build_label_vote(LabelVoteInput {
                training_candidate_id: candidate.clone(),
                task: "structured_fact_semantics".into(),
                labeler: labeler(id),
                outcome,
                observations: vec![observation.clone()],
                reason_codes: vec![if applicable {
                    "exact_reviewed_source_measurement".into()
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
                "adafruit-tps61023-training-example",
                &label.training_label_id,
                &source,
            )),
            task: "structured_fact_semantics".into(),
            document_state_id: None,
            document_ir_logical_sha256: None,
            datasheet_sketch_digest: None,
            observation_ids: vec![observation.observation_id],
            source_record_ids: vec![source],
            candidate_ids: vec![candidate],
            training_label_record_id: label.training_label_id.clone(),
            document_family_id: "manufacturer.adafruit.tps61023.github-readme".into(),
            template_family_id: Some("adafruit.github-hardware-readme.bench-measurements".into()),
            manufacturer_key: Some("adafruit".into()),
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
        schema_version: "fdx4-adafruit-tps61023-supervision-bundle/1",
        source_artifact_sha256: SOURCE_SHA,
        source_license_sha256: LICENSE_SHA,
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
            "single_adafruit_readme_revision".into(),
            "board_publisher_bench_measurements_not_component_limits".into(),
            "not_final_blind".into(),
        ],
    };
    let mut bytes = serde_json::to_vec_pretty(&bundle).unwrap();
    bytes.push(b'\n');
    fs::write(Path::new(&output), bytes).unwrap();
}
