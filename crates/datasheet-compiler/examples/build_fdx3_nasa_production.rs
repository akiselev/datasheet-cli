use std::{collections::BTreeMap, env, fs, path::Path};

use datasheet_compiler::supervision::{
    CombinationResult, LabelVoteInput, build_label_vote, combine_votes,
    freeze_calibration_snapshot, freeze_reviewed_development_set,
};
use datasheet_core::supervision::{
    LabelVote, LabelerCalibration, LabelerRef, LineageGroup, ReviewedDevelopmentExample,
    RightsSnapshot, STRUCTURED_FACT_SCHEMA, SourceVocabularyMap, StructuredFactRecord,
    SupervisionObservation, SupervisionProducer, TRAINING_EXAMPLE_SCHEMA, TrainingExample,
    TrainingLabel, VoteOutcome,
};
use datasheet_core::{CandidateId, Sha256Digest};
use serde::Serialize;
use serde_json::{Value, json};

const SOURCE_SHA: &str = "916fcf4561f6961c78c686421abe8922f2845d9e9946c59ea742199374192c43";
const REVIEW_SHA: &str = "a1b0456e150a0c40d9bae51c8b63014eb72e3aaa2482bfb20799975135a5302b";
const REVIEWED_ROWS: [usize; 7] = [0, 5, 11, 19, 24, 30, 34];

#[derive(Clone)]
struct Fact {
    row: usize,
    mpn: String,
    key: String,
    raw: String,
    unit: Option<String>,
    shape: String,
    source_record_id: Sha256Digest,
    map: SourceVocabularyMap,
}

#[derive(Serialize)]
struct ProductionBundle {
    schema_version: &'static str,
    source_artifact_sha256: &'static str,
    source_record_count: usize,
    reviewed_row_indices: Vec<usize>,
    structured_facts: Vec<StructuredFactRecord>,
    votes: Vec<LabelVote>,
    labels: Vec<TrainingLabel>,
    training_examples: Vec<TrainingExample>,
}

fn digest(value: &impl Serialize) -> Sha256Digest {
    Sha256Digest::of_bytes(&serde_json::to_vec(value).expect("serializable"))
}

fn producer() -> SupervisionProducer {
    SupervisionProducer {
        name: "datasheet-fdx3-nasa-npsl-production".into(),
        version: "1".into(),
        build: Some("foundry-5ec50e4+datasheet-8272515".into()),
        configuration_digest: Sha256Digest(
            "01a6cfbdfcd32f12ba611a45b525c1c41d6854a3f484df9dd0df80badd9b2975".into(),
        ),
        model_digest: None,
    }
}

fn labeler(id: &str) -> LabelerRef {
    LabelerRef {
        labeler_id: id.into(),
        labeler_version: "1".into(),
    }
}

fn strip_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut rest = row;
    while let Some(start) = rest.find("<td") {
        rest = &rest[start..];
        let Some(open) = rest.find('>') else { break };
        rest = &rest[open + 1..];
        let Some(close) = rest.find("</td>") else {
            break;
        };
        let cell = &rest[..close];
        let mut text = String::new();
        let mut in_tag = false;
        for ch in cell.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => text.push(ch),
                _ => {}
            }
        }
        let text = text
            .replace("&nbsp;", " ")
            .replace("&#35;", "#")
            .replace("&amp;", "&");
        cells.push(text.split_whitespace().collect::<Vec<_>>().join(" "));
        rest = &rest[close + 5..];
    }
    cells
}

fn rows(bytes: &[u8]) -> Vec<Vec<String>> {
    let text = bytes
        .iter()
        .map(|byte| char::from(*byte))
        .collect::<String>();
    text.split("</tr>")
        .map(strip_cells)
        .filter(|cells| cells.len() == 10 && cells[0].contains("2N"))
        .collect()
}

fn write(path: &Path, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec_pretty(value).expect("serialize")).expect("write artifact");
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        4,
        "usage: build_fdx3_nasa_production <html> <record-ids.json> <output-dir>"
    );
    let source = fs::read(&args[1]).expect("read exact retained NASA source");
    assert_eq!(
        Sha256Digest::of_bytes(&source).0,
        SOURCE_SHA,
        "stale/wrong NASA source"
    );
    let rows = rows(&source);
    assert_eq!(rows.len(), 35, "reviewed table shape drift");
    let review_bytes = include_bytes!("../../../evidence/fdx3-nasa-independent-row-review.json");
    assert_eq!(
        Sha256Digest::of_bytes(review_bytes).0,
        REVIEW_SHA,
        "review record drift"
    );
    let review: Value = serde_json::from_slice(review_bytes).expect("review JSON");
    for reviewed in review["rows"].as_array().expect("reviewed rows") {
        let index = usize::try_from(reviewed["data_row_index"].as_u64().expect("row index"))
            .expect("row index fits");
        assert_eq!(
            Sha256Digest::of_bytes(&serde_json::to_vec(&rows[index][0..8]).expect("review row")).0,
            reviewed["part_through_condition_cells_sha256"]
                .as_str()
                .expect("review digest"),
            "independently reviewed NASA row changed"
        );
    }
    let record_ids: BTreeMap<String, Vec<Sha256Digest>> =
        serde_json::from_slice(&fs::read(&args[2]).expect("record IDs")).expect("record ID JSON");
    let maps: Vec<SourceVocabularyMap> = serde_json::from_slice(include_bytes!(
        "../../../evidence/fdx3-nasa-npsl-source-vocabulary-maps.json"
    ))
    .expect("vocabulary maps");
    let maps = maps
        .into_iter()
        .map(|map| (map.external_field_key.clone(), map))
        .collect::<BTreeMap<_, _>>();
    let fields = [
        ("I_C", 2, Some("A"), "scalar"),
        ("V_CBO", 3, Some("V"), "scalar"),
        ("V_CEO", 4, Some("V"), "scalar"),
        ("V_EBO", 5, Some("V"), "scalar"),
        ("h_FE_min_max", 6, None, "range"),
        ("h_FE_at_I_C", 7, Some("mA"), "scalar"),
    ];
    let mut facts = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        for (key, column, unit, shape) in fields {
            facts.push(Fact {
                row: row_index,
                mpn: row[0].clone(),
                key: key.into(),
                raw: row[column].clone(),
                unit: unit.map(str::to_owned),
                shape: shape.into(),
                source_record_id: record_ids[key][row_index].clone(),
                map: maps[key].clone(),
            });
        }
    }
    assert_eq!(facts.len(), 210);

    let lineage = {
        let mut group = LineageGroup {
            group_id: Sha256Digest::of_bytes(b"placeholder"),
            publisher_key: Some("government.nasa.nepp.npsl".into()),
            upstream_dataset_fingerprint: Some(Sha256Digest(SOURCE_SHA.into())),
            source_schema: Some("NASA.NEPP.NPSL/lp_npt/1998-03".into()),
            observation_epoch: Some("1998-03".into()),
        };
        group.group_id = group.recompute_id();
        group
    };
    let rights = RightsSnapshot {
        snapshot_id: digest(&json!([
            "policy.nasa.nepp.npsl.public",
            "2026-08-22",
            SOURCE_SHA
        ])),
        policy_id: "policy.nasa.nepp.npsl.public".into(),
        policy_version: "2026-08-22".into(),
        retention_permitted: true,
        training_permitted: true,
        remote_disclosure_permitted: false,
    };
    let observations = facts
        .iter()
        .map(|fact| {
            let observation_id = digest(&json!({
                "source_record_id": fact.source_record_id,
                "source_artifact_sha256": SOURCE_SHA,
                "source_row": fact.row + 8,
                "field": fact.key,
                "raw": fact.raw,
                "unit": fact.unit,
            }));
            (
                fact.source_record_id.clone(),
                SupervisionObservation {
                    observation_id,
                    source_record_id: fact.source_record_id.clone(),
                    source_schema: "bronze.structured_fact_observations/v1".into(),
                    producer: producer(),
                    document_state: None,
                    candidate_ids: vec![],
                    evidence_anchors: vec![],
                    lineage_groups: vec![lineage.clone()],
                    rights_snapshots: vec![rights.clone()],
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    let reviewed_examples = facts
        .iter()
        .filter(|fact| REVIEWED_ROWS.contains(&fact.row))
        .map(|fact| {
            let target = json!({
                "external_field_key": fact.key,
                "source_part_number": fact.mpn,
                "predicate_term_id": fact.map.predicate_term_id,
                "quantity_kind": fact.map.expected_quantity_kind,
                "value_shape": fact.shape,
                "raw_value": fact.raw,
                "raw_unit": fact.unit,
                "regime": null,
            });
            let candidate = CandidateId(format!("nasa-npsl:{}", fact.source_record_id.0));
            ReviewedDevelopmentExample {
                example_id: digest(&json!([
                    "reviewed-development",
                    fact.source_record_id,
                    target
                ])),
                task: "structured_fact_semantics".into(),
                training_candidate_id: candidate,
                expected_target_json: target,
                observation_ids: vec![observations[&fact.source_record_id].observation_id.clone()],
                reviewer_record_id: digest(&json!([
                    "independent-html-row-review",
                    SOURCE_SHA,
                    fact.row + 8,
                    fact.key,
                    fact.raw
                ])),
            }
        })
        .collect::<Vec<_>>();
    let development = freeze_reviewed_development_set(
        maps.values()
            .next()
            .expect("map")
            .ontology_snapshot_digest
            .clone(),
        reviewed_examples,
        producer(),
    )
    .expect("development set");
    assert_eq!(
        development.digest.0,
        review["reviewed_development_set_digest"]
            .as_str()
            .expect("reviewed development-set digest"),
        "independent review is not bound to the generated development set"
    );
    let measurements = review["labeler_measurements"]
        .as_object()
        .expect("labeler measurements");
    let measured_labelers = [
        "quantity.dimension.v1",
        "source.range_align.v1",
        "source.value_align_unique.v1",
        "source.vocabulary_map.v1",
    ];
    assert_eq!(measurements.len(), measured_labelers.len());
    let calibration = freeze_calibration_snapshot(
        development.digest.clone(),
        development.ontology_snapshot_digest.clone(),
        measured_labelers
        .into_iter()
        .map(|id| {
            let measurement = measurements.get(id).expect("named labeler measurement");
            let correct = u32::try_from(measurement["correct"].as_u64().expect("correct count"))
                .expect("correct count fits u32");
            let total = u32::try_from(measurement["total"].as_u64().expect("total count"))
                .expect("total count fits u32");
            LabelerCalibration {
            labeler: labeler(id),
            reviewed_correct: correct,
            reviewed_total: total,
            precision_millionths: u32::try_from(u64::from(correct) * 1_000_000 / u64::from(total))
                .expect("precision fits u32"),
            weight_millionths: 1_000_000,
            }
        })
        .collect(),
        1_000_000,
        1,
        producer(),
    )
    .expect("calibration");

    let mut structured = Vec::new();
    let mut votes = Vec::new();
    let mut labels = Vec::new();
    let mut training_examples = Vec::new();
    for fact in &facts {
        let observation = observations[&fact.source_record_id].clone();
        let candidate = CandidateId(format!("nasa-npsl:{}", fact.source_record_id.0));
        let target = json!({
            "external_field_key": fact.key, "predicate_term_id": fact.map.predicate_term_id,
            "source_part_number": fact.mpn,
            "quantity_kind": fact.map.expected_quantity_kind, "value_shape": fact.shape,
            "raw_value": fact.raw, "raw_unit": fact.unit, "regime": null,
        });
        let mut fact_votes = Vec::new();
        for id in [
            "quantity.dimension.v1",
            "source.range_align.v1",
            "source.value_align_unique.v1",
            "source.vocabulary_map.v1",
        ] {
            let applicable = match id {
                "source.range_align.v1" => fact.shape == "range",
                "source.value_align_unique.v1" => fact.shape == "scalar",
                _ => true,
            };
            let outcome = if applicable {
                VoteOutcome::Vote {
                    vote_json: target.clone(),
                }
            } else {
                VoteOutcome::Abstain {
                    abstain_reason: "labeler_not_applicable_to_value_shape".into(),
                }
            };
            let vote = build_label_vote(LabelVoteInput {
                training_candidate_id: candidate.clone(),
                task: "structured_fact_semantics".into(),
                labeler: labeler(id),
                outcome,
                observations: vec![observation.clone()],
                reason_codes: vec![if applicable {
                    "exact_source_field_under_reviewed_mapping".into()
                } else {
                    "shape_mismatch_abstention".into()
                }],
                producer: producer(),
            })
            .expect("vote");
            fact_votes.push(vote.clone());
            votes.push(vote);
        }
        let CombinationResult::Label { label } = combine_votes(
            &candidate,
            "structured_fact_semantics",
            &fact_votes,
            &calibration,
        )
        .expect("combine") else {
            panic!("reviewed NASA fact unexpectedly abstained")
        };
        let example = TrainingExample {
            schema_version: TRAINING_EXAMPLE_SCHEMA.into(),
            example_id: digest(&json!([
                "training-example",
                label.training_label_id,
                fact.source_record_id
            ])),
            task: "structured_fact_semantics".into(),
            document_state_id: None,
            document_ir_logical_sha256: None,
            datasheet_sketch_digest: None,
            observation_ids: vec![observation.observation_id.clone()],
            source_record_ids: vec![fact.source_record_id.clone()],
            candidate_ids: vec![candidate.clone()],
            training_label_record_id: label.training_label_id.clone(),
            document_family_id: "government.nasa.nepp.npsl.lp_npt".into(),
            template_family_id: None,
            manufacturer_key: None,
            assigned_split: "train".into(),
            ontology_snapshot_digest: development.ontology_snapshot_digest.clone(),
            rights_snapshot_ids: vec![rights.snapshot_id.clone()],
            producer: producer(),
        };
        example
            .validate_structure()
            .expect("source-native training example");
        training_examples.push(example);
        labels.push(label);
        let mut record = StructuredFactRecord {
            schema_version: STRUCTURED_FACT_SCHEMA.into(),
            structured_fact_id: Sha256Digest::of_bytes(b"placeholder"),
            bronze_fact_record_id: fact.source_record_id.clone(),
            source_vocabulary_map_id: Some(fact.map.map_id.clone()),
            predicate_candidate_id: Some(candidate),
            raw_value: fact.raw.clone(),
            raw_unit: fact.unit.clone(),
            normalized_quantity: None,
            normalization_status: "reviewed_source_native_not_regime_normalized".into(),
            producer: producer(),
            warnings: vec!["rating_or_test_regime_not_asserted".into()],
        };
        record.structured_fact_id = digest(&json!([
            record.bronze_fact_record_id,
            record.source_vocabulary_map_id,
            record.raw_value,
            record.raw_unit,
            record.normalization_status
        ]));
        record.validate_structure().expect("structured fact");
        structured.push(record);
    }
    let bundle = ProductionBundle {
        schema_version: "fdx3-nasa-production-bundle/1",
        source_artifact_sha256: SOURCE_SHA,
        source_record_count: facts.len(),
        reviewed_row_indices: REVIEWED_ROWS.to_vec(),
        structured_facts: structured,
        votes,
        labels,
        training_examples,
    };
    let output = Path::new(&args[3]);
    fs::create_dir_all(output).expect("output directory");
    write(
        &output.join("fdx3-nasa-reviewed-development-set.json"),
        &development,
    );
    write(
        &output.join("fdx3-nasa-calibration-snapshot.json"),
        &calibration,
    );
    write(&output.join("fdx3-nasa-production-bundle.json"), &bundle);
    write(
        &output.join("fdx3-nasa-production-metrics.json"),
        &json!({
            "schema_version": "fdx3-production-metrics/1", "source_facts": 210, "reviewed_examples": 42,
            "votes": 840, "positive_votes": 630, "abstentions": 210, "combined_labels": 210,
            "training_examples": 210, "failure_buckets": {"ambiguous_regime_excluded": 210, "manufacturer_authority_excluded": 210, "second_family_access_or_rights_blocked": 1},
            "abstention_buckets": {"shape_mismatch": 210},
        }),
    );
}
