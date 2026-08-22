use std::{collections::BTreeMap, env, fs, path::Path};

use datasheet_compiler::corpus::freeze_protected_corpus_v2;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusEvidenceClass, CorpusInputBinding, CorpusMemberV2, GroupingLevel,
    PROTECTED_CORPUS_V2_SCHEMA, ProtectedCorpusManifestV2, ProtectedSliceManifestV2,
    ProtectedSplit,
};
use serde_json::{Value, json};

const NASA_BUNDLE_SHA: &str = "5f859c8944f3306f5219f1aea8aa7f545c3e815100e88bd1e523dd66b63fb892";
const TPX_BUNDLE_SHA: &str = "6a7a6cfa69e619fd048e9a5a16aebda480eb1b6bb2ab92000bd63edcfcdfe29f";
const FOUNDRY_REVISION: &str = "d4e35dda5fa1f1b78347a4fec2e26b1b06c3e718";

fn digest(value: &impl serde::Serialize) -> Sha256Digest {
    Sha256Digest::of_bytes(&serde_json::to_vec(value).expect("serializable"))
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        4,
        "usage: build_fdx4_protected_corpus_v2 <nasa-bundle> <tpx-bundle> <output>"
    );
    let nasa_bytes = fs::read(&args[1]).unwrap();
    let tpx_bytes = fs::read(&args[2]).unwrap();
    assert_eq!(
        Sha256Digest::of_bytes(&nasa_bytes).0,
        NASA_BUNDLE_SHA,
        "wrong audited NASA bundle"
    );
    assert_eq!(
        Sha256Digest::of_bytes(&tpx_bytes).0,
        TPX_BUNDLE_SHA,
        "wrong reviewed TPX bundle"
    );
    let nasa: Value = serde_json::from_slice(&nasa_bytes).unwrap();
    let tpx: Value = serde_json::from_slice(&tpx_bytes).unwrap();
    let nasa_observations = nasa["votes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|vote| {
            let observation = &vote["observations"][0];
            (
                observation["source_record_id"].as_str().unwrap(),
                observation,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let nasa_near = digest(&(
        "near-duplicate",
        NASA_BUNDLE_SHA,
        "NASA.NEPP.NPSL/lp_npt/1998-03",
    ));
    let nasa_revision = digest(&(
        "revision",
        "916fcf4561f6961c78c686421abe8922f2845d9e9946c59ea742199374192c43",
    ));
    let mut members = nasa["training_examples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|example| {
            let source = example["source_record_ids"][0].as_str().unwrap();
            let observation = nasa_observations[source];
            CorpusMemberV2 {
                example_id: serde_json::from_value(example["example_id"].clone()).unwrap(),
                example_content_sha256: digest(example),
                evidence_class: CorpusEvidenceClass::RealDocument,
                real_world_generalization_eligible: true,
                document_family_id: "government.nasa.nepp.npsl.lp_npt".into(),
                template_family_id: Some("nasa-nepp-npsl.html-table.lp-npt".into()),
                // This is an unresolved grouping cohort, not a manufacturer-authority claim.
                manufacturer_key: Some("nasa-npsl:unresolved-manufacturer-cohort".into()),
                lineage_group_ids: observation["lineage_groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|group| serde_json::from_value(group["group_id"].clone()).unwrap())
                    .collect(),
                near_duplicate_group_id: nasa_near.clone(),
                revision_group_id: Some(nasa_revision.clone()),
                synthetic_root_example_id: None,
                rights_snapshot_ids: example["rights_snapshot_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| serde_json::from_value(id.clone()).unwrap())
                    .collect(),
                training_permitted: true,
                restriction_codes: vec![],
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(members.len(), 210);

    let tpx_lineage = digest(&(
        "publisher-lineage",
        "publisher.arduino",
        "docs-content",
        "ad72210",
    ));
    let tpx_near = digest(&("near-duplicate", "TPX00242", "docs-content-ad72210"));
    let tpx_revision = digest(&(
        "revision",
        "TPX00242",
        "ad72210c6dffa698374d4f834aae2d568e49b41d",
    ));
    let tpx_rights = digest(&(
        "policy.arduino.docs.cc-by-sa",
        "2026-08-22",
        "2cb2dff3c03952804b7b0abc0648289242c17cf34d829ad94ad2b796adeae284",
    ));
    let tpx_members = tpx["training_examples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|example| CorpusMemberV2 {
            example_id: serde_json::from_value(example["example_id"].clone()).unwrap(),
            example_content_sha256: digest(example),
            evidence_class: CorpusEvidenceClass::RealDocument,
            real_world_generalization_eligible: true,
            document_family_id: "manufacturer.arduino.tpx00242.docs-content".into(),
            template_family_id: Some("arduino.docs-content.accessory-datasheet".into()),
            manufacturer_key: Some("arduino".into()),
            lineage_group_ids: vec![tpx_lineage.clone()],
            near_duplicate_group_id: tpx_near.clone(),
            revision_group_id: Some(tpx_revision.clone()),
            synthetic_root_example_id: None,
            rights_snapshot_ids: vec![tpx_rights.clone()],
            training_permitted: true,
            restriction_codes: vec![],
        })
        .collect::<Vec<_>>();
    assert_eq!(tpx_members.len(), 55);
    let tpx_ids = tpx_members
        .iter()
        .map(|member| member.example_id.clone())
        .collect::<Vec<_>>();
    members.extend(tpx_members);

    // Independently rooted minimized case. Its bytes are persisted outside the source tree;
    // only identity/grouping metadata enters the public manifest.
    let synthetic_payload = json!({"schema_version":"fdx4-independent-synthetic-case/1","case":"bounded-range-cell",
        "input":{"raw":"4.75 - 5.25 V","header":"Output voltage"},"expected_contract":"range_candidate_with_owned_condition"});
    let synthetic_content = digest(&synthetic_payload);
    let synthetic_id = digest(&("independently-synthetic-example", &synthetic_content));
    members.push(CorpusMemberV2 {
        example_id: synthetic_id.clone(),
        example_content_sha256: synthetic_content.clone(),
        evidence_class: CorpusEvidenceClass::IndependentlySynthetic,
        real_world_generalization_eligible: false,
        document_family_id: "synthetic.independent.contract.range-cell".into(),
        template_family_id: Some("synthetic.independent.table-v1".into()),
        manufacturer_key: Some("synthetic:not-a-manufacturer".into()),
        lineage_group_ids: vec![digest(&(
            "independent-synthetic-lineage",
            &synthetic_content,
        ))],
        near_duplicate_group_id: digest(&("independent-synthetic-near", &synthetic_content)),
        revision_group_id: Some(digest(&(
            "independent-synthetic-revision",
            &synthetic_content,
        ))),
        synthetic_root_example_id: None,
        rights_snapshot_ids: vec![digest(&(
            "synthetic-fixture-policy",
            "generated-in-repository",
            1_u8,
        ))],
        training_permitted: true,
        restriction_codes: vec![],
    });

    let train_ids = members
        .iter()
        .take(210)
        .map(|member| member.example_id.clone())
        .collect::<Vec<_>>();
    let all_groups = vec![
        GroupingLevel::ExactContent,
        GroupingLevel::NearDuplicate,
        GroupingLevel::Revision,
        GroupingLevel::SyntheticRoot,
        GroupingLevel::DocumentFamily,
        GroupingLevel::TemplateFamily,
        GroupingLevel::Manufacturer,
        GroupingLevel::PublisherLineage,
    ];
    let mut development_ids = tpx_ids.clone();
    development_ids.push(synthetic_id.clone());
    let slices = ProtectedSplit::ALL
        .into_iter()
        .map(|split| {
            let (member_ids, groups, limitations) = match split {
                ProtectedSplit::Train => (train_ids.clone(), vec![], vec![]),
                ProtectedSplit::Development => (
                    development_ids.clone(),
                    all_groups.clone(),
                    vec![
                        "visible_synthetic_contract_case_included".into(),
                        "single_real_heldout_document_not_representative".into(),
                        "evaluation_tags_declared_to_overlap".into(),
                    ],
                ),
                ProtectedSplit::TemplateHeldOut
                | ProtectedSplit::DocumentFamilyHeldOut
                | ProtectedSplit::ManufacturerHeldOut => (
                    tpx_ids.clone(),
                    all_groups.clone(),
                    vec![
                        "single_real_heldout_document_not_representative".into(),
                        "evaluation_tags_declared_to_overlap".into(),
                    ],
                ),
                ProtectedSplit::FinalBlindTest => (
                    vec![],
                    all_groups.clone(),
                    vec![
                        "no_independently_hidden_artifact".into(),
                        "repository_visible_synthetic_case_not_admitted".into(),
                    ],
                ),
            };
            ProtectedSliceManifestV2 {
                split,
                member_ids,
                held_out_from_train_by: groups,
                hidden: split == ProtectedSplit::FinalBlindTest,
                artifact_digest: digest(&"placeholder"),
                supports_real_world_generalization: false,
                limitation_codes: limitations,
            }
        })
        .collect();
    let manifest = freeze_protected_corpus_v2(ProtectedCorpusManifestV2 {
        schema_version: PROTECTED_CORPUS_V2_SCHEMA.into(),
        corpus_digest: digest(&"placeholder"),
        datasheet_revision: "fdx4-overlap-open-docs".into(),
        foundry_revision: FOUNDRY_REVISION.into(),
        ontology_snapshot_digest: serde_json::from_value(tpx["vocabulary"]["digest"].clone())
            .unwrap(),
        calibration_snapshot_digest: serde_json::from_value(
            tpx["calibration_snapshot"]["digest"].clone(),
        )
        .unwrap(),
        builder_configuration_digest: digest(&(
            "fdx4-protected-split-builder",
            2_u8,
            "per-slice-train-disjoint-overlap-aware",
        )),
        inputs: vec![
            CorpusInputBinding {
                role: "audited_fdx3_nasa_training_examples".into(),
                content_sha256: Sha256Digest(NASA_BUNDLE_SHA.into()),
                logical_id: Some("government.nasa.nepp.npsl.lp_npt".into()),
                row_count: 210,
            },
            CorpusInputBinding {
                role: "reviewed_arduino_tpx00242_examples".into(),
                content_sha256: Sha256Digest(TPX_BUNDLE_SHA.into()),
                logical_id: Some("manufacturer.arduino.tpx00242.docs-content-ad72210".into()),
                row_count: 55,
            },
            CorpusInputBinding {
                role: "independently_rooted_synthetic_contract_case".into(),
                content_sha256: synthetic_content,
                logical_id: Some("synthetic.independent.range-cell/1".into()),
                row_count: 1,
            },
        ],
        members,
        exclusions: vec![],
        slices,
        contract_plumbing_ready: false,
        model_qualification_ready: false,
        limitation_codes: vec![
            "only_one_real_evaluation_document_family".into(),
            "evaluation_tags_overlap_by_declaration".into(),
            "final_blind_is_empty".into(),
            "repository_visible_synthetic_is_development_only".into(),
            "no_real_world_generalization_claim".into(),
            "fdx5_model_qualification_blocked".into(),
        ],
    })
    .unwrap();
    let mut bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    bytes.push(b'\n');
    fs::write(Path::new(&args[3]), bytes).unwrap();
}
