use std::{collections::BTreeMap, env, fs, path::Path};

use datasheet_compiler::corpus::freeze_protected_corpus;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusInputBinding, CorpusMember, PROTECTED_CORPUS_SCHEMA, ProtectedCorpusManifest,
    ProtectedSplit, ProtectedSplitManifest,
};
use serde_json::{Value, json};

const FDX3_BUNDLE_SHA: &str = "5f859c8944f3306f5219f1aea8aa7f545c3e815100e88bd1e523dd66b63fb892";
const DATASHEET_REVISION: &str = "430d487b882e6c1d0bf3d6962525c4a283789283";
const FOUNDRY_REVISION: &str = "bb7f197b3551bd3fd58e9db668230baf879dcb77";

fn digest(value: &impl serde::Serialize) -> Sha256Digest {
    Sha256Digest::of_bytes(&serde_json::to_vec(value).expect("serializable"))
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        3,
        "usage: build_fdx4_protected_corpus <fdx3-bundle> <output>"
    );
    let bytes = fs::read(&args[1]).expect("FDX3 bundle");
    assert_eq!(
        Sha256Digest::of_bytes(&bytes).0,
        FDX3_BUNDLE_SHA,
        "wrong audited FDX3 bundle"
    );
    let bundle: Value = serde_json::from_slice(&bytes).expect("FDX3 JSON");
    let votes = bundle["votes"].as_array().expect("votes");
    let observations = votes
        .iter()
        .map(|vote| {
            let observation = &vote["observations"][0];
            (
                observation["source_record_id"].as_str().unwrap(),
                observation,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let examples = bundle["training_examples"].as_array().expect("examples");
    assert_eq!(examples.len(), 210);
    assert!(
        examples
            .iter()
            .all(|example| example["assigned_split"] == "train")
    );
    let near_duplicate_group_id = digest(&json!([
        "near-duplicate",
        FDX3_BUNDLE_SHA,
        "NASA.NEPP.NPSL/lp_npt/1998-03"
    ]));
    let revision_group_id = digest(&json!([
        "revision",
        "916fcf4561f6961c78c686421abe8922f2845d9e9946c59ea742199374192c43"
    ]));
    let members = examples
        .iter()
        .map(|example| {
            let source = example["source_record_ids"][0].as_str().unwrap();
            let observation = observations[source];
            CorpusMember {
                example_id: serde_json::from_value(example["example_id"].clone()).unwrap(),
                example_content_sha256: digest(example),
                assigned_split: ProtectedSplit::Train,
                document_family_id: example["document_family_id"].as_str().unwrap().into(),
                template_family_id: None,
                manufacturer_key: None,
                lineage_group_ids: observation["lineage_groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|group| serde_json::from_value(group["group_id"].clone()).unwrap())
                    .collect(),
                near_duplicate_group_id: near_duplicate_group_id.clone(),
                revision_group_id: Some(revision_group_id.clone()),
                synthetic_root_example_id: None,
                rights_snapshot_ids: example["rights_snapshot_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| serde_json::from_value(id.clone()).unwrap())
                    .collect(),
                training_permitted: observation["rights_snapshots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|rights| {
                        rights["retention_permitted"] == true
                            && rights["training_permitted"] == true
                    }),
                restriction_codes: vec![],
            }
        })
        .collect::<Vec<_>>();
    let train_ids = members
        .iter()
        .map(|member| member.example_id.clone())
        .collect::<Vec<_>>();
    let splits = ProtectedSplit::ALL
        .into_iter()
        .map(|split| {
            let (member_ids, hidden, limitations) = if split == ProtectedSplit::Train {
                (train_ids.clone(), false, vec![])
            } else {
                let limitation = match split {
                    ProtectedSplit::Development => "insufficient_distinct_development_group",
                    ProtectedSplit::TemplateHeldOut => "no_distinct_template_family",
                    ProtectedSplit::DocumentFamilyHeldOut => "no_second_document_family",
                    ProtectedSplit::ManufacturerHeldOut => "manufacturer_authority_absent",
                    ProtectedSplit::FinalBlindTest => "no_hidden_rights_cleared_family",
                    ProtectedSplit::Train => unreachable!(),
                };
                (
                    vec![],
                    split == ProtectedSplit::FinalBlindTest,
                    vec![limitation.into()],
                )
            };
            ProtectedSplitManifest {
                split,
                member_ids,
                hidden,
                artifact_digest: Sha256Digest::of_bytes(b"placeholder"),
                limitation_codes: limitations,
            }
        })
        .collect();
    let manifest = freeze_protected_corpus(ProtectedCorpusManifest {
        schema_version: PROTECTED_CORPUS_SCHEMA.into(),
        corpus_digest: Sha256Digest::of_bytes(b"placeholder"),
        datasheet_revision: DATASHEET_REVISION.into(),
        foundry_revision: FOUNDRY_REVISION.into(),
        ontology_snapshot_digest: Sha256Digest(
            "942c84a70c2ae0c45aa5665ecb85e03abbefdbf8fdd20760dedc99594d27511e".into(),
        ),
        calibration_snapshot_digest: Sha256Digest(
            "72558d93b74ffdd9c85bf7013261a4b01aa15fdaf22ddbf758f3719bf9101040".into(),
        ),
        builder_configuration_digest: digest(&json!([
            "fdx4-protected-split-builder",
            1,
            "all-related-groups-stay-together"
        ])),
        inputs: vec![CorpusInputBinding {
            role: "audited_fdx3_nasa_production_bundle".into(),
            content_sha256: Sha256Digest(FDX3_BUNDLE_SHA.into()),
            logical_id: Some("government.nasa.nepp.npsl.lp_npt".into()),
            row_count: 210,
        }],
        members,
        exclusions: vec![],
        splits,
        model_qualification_ready: false,
        limitation_codes: vec![
            "single_real_document_family".into(),
            "single_publisher_lineage".into(),
            "manufacturer_authority_absent".into(),
            "no_hidden_evaluation_artifact".into(),
            "fdx5_blocked_pending_protected_evaluation_groups".into(),
        ],
    })
    .expect("protected corpus");
    let mut output = serde_json::to_vec_pretty(&manifest).unwrap();
    output.push(b'\n');
    fs::write(Path::new(&args[2]), output).expect("write manifest");
}
