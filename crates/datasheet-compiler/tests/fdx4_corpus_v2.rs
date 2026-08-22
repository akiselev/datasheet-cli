use datasheet_compiler::corpus::freeze_protected_corpus_v2;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusEvidenceClass, CorpusInputBinding, CorpusMemberV2, GroupingLevel,
    PROTECTED_CORPUS_V2_SCHEMA, ProtectedCorpusManifestV2, ProtectedSliceManifestV2,
    ProtectedSplit,
};

fn digest(byte: char) -> Sha256Digest {
    Sha256Digest(std::iter::repeat_n(byte, 64).collect())
}

fn member(id: char, class: CorpusEvidenceClass) -> CorpusMemberV2 {
    CorpusMemberV2 {
        example_id: digest(id),
        example_content_sha256: digest(char::from_u32(u32::from(id) + 1).unwrap()),
        evidence_class: class,
        real_world_generalization_eligible: class == CorpusEvidenceClass::RealDocument,
        document_family_id: format!("document-{id}"),
        template_family_id: Some(format!("template-{id}")),
        manufacturer_key: Some(format!("manufacturer-{id}")),
        lineage_group_ids: vec![digest(char::from_u32(u32::from(id) + 2).unwrap())],
        near_duplicate_group_id: digest(char::from_u32(u32::from(id) + 3).unwrap()),
        revision_group_id: Some(digest(char::from_u32(u32::from(id) + 4).unwrap())),
        synthetic_root_example_id: None,
        rights_snapshot_ids: vec![digest('f')],
        training_permitted: true,
        restriction_codes: vec![],
    }
}

fn fixture() -> ProtectedCorpusManifestV2 {
    let training = member('1', CorpusEvidenceClass::RealDocument);
    let held_out = member('5', CorpusEvidenceClass::RealDocument);
    let mut synthetic = member('a', CorpusEvidenceClass::IndependentlySynthetic);
    synthetic.real_world_generalization_eligible = false;
    let baseline = vec![
        GroupingLevel::ExactContent,
        GroupingLevel::NearDuplicate,
        GroupingLevel::Revision,
        GroupingLevel::SyntheticRoot,
    ];
    let slices = ProtectedSplit::ALL
        .into_iter()
        .map(|split| {
            let (member_ids, held_out_from_train_by, supports, limitations) = match split {
                ProtectedSplit::Train => (vec![training.example_id.clone()], vec![], false, vec![]),
                ProtectedSplit::Development => (
                    vec![held_out.example_id.clone()],
                    baseline.clone(),
                    true,
                    vec![],
                ),
                ProtectedSplit::TemplateHeldOut => {
                    let mut groups = baseline.clone();
                    groups.push(GroupingLevel::TemplateFamily);
                    (vec![held_out.example_id.clone()], groups, true, vec![])
                }
                ProtectedSplit::DocumentFamilyHeldOut => {
                    let mut groups = baseline.clone();
                    groups.push(GroupingLevel::DocumentFamily);
                    (vec![held_out.example_id.clone()], groups, true, vec![])
                }
                ProtectedSplit::ManufacturerHeldOut => {
                    let mut groups = baseline.clone();
                    groups.push(GroupingLevel::Manufacturer);
                    (vec![held_out.example_id.clone()], groups, true, vec![])
                }
                ProtectedSplit::FinalBlindTest => (
                    vec![synthetic.example_id.clone()],
                    baseline.clone(),
                    false,
                    vec!["independently_synthetic_contract_only".into()],
                ),
            };
            ProtectedSliceManifestV2 {
                split,
                member_ids,
                held_out_from_train_by,
                hidden: split == ProtectedSplit::FinalBlindTest,
                artifact_digest: digest('0'),
                supports_real_world_generalization: supports,
                limitation_codes: limitations,
            }
        })
        .collect();
    ProtectedCorpusManifestV2 {
        schema_version: PROTECTED_CORPUS_V2_SCHEMA.into(),
        corpus_digest: digest('0'),
        datasheet_revision: "fixture".into(),
        foundry_revision: "fixture".into(),
        ontology_snapshot_digest: digest('2'),
        calibration_snapshot_digest: digest('3'),
        builder_configuration_digest: digest('4'),
        inputs: vec![CorpusInputBinding {
            role: "examples".into(),
            content_sha256: digest('5'),
            logical_id: None,
            row_count: 3,
        }],
        members: vec![training, held_out, synthetic],
        exclusions: vec![],
        slices,
        contract_plumbing_ready: true,
        model_qualification_ready: false,
        limitation_codes: vec!["synthetic_final_blind_cannot_qualify".into()],
    }
}

fn production() -> ProtectedCorpusManifestV2 {
    serde_json::from_str(include_str!(
        "../../../evidence/fdx4-protected-corpus-manifest-v2.json"
    ))
    .unwrap()
}

#[test]
fn production_manifest_has_honest_overlap_and_hidden_isolation() {
    let manifest = production();
    manifest.validate().unwrap();
    assert_eq!(manifest.members.len(), 275);
    let train = manifest
        .slices
        .iter()
        .find(|slice| slice.split == ProtectedSplit::Train)
        .unwrap();
    assert_eq!(train.member_ids.len(), 210);
    let evaluation = manifest
        .slices
        .iter()
        .filter(|slice| {
            matches!(
                slice.split,
                ProtectedSplit::Development
                    | ProtectedSplit::TemplateHeldOut
                    | ProtectedSplit::DocumentFamilyHeldOut
                    | ProtectedSplit::ManufacturerHeldOut
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(evaluation[0].member_ids.len(), 65);
    assert!(
        evaluation[1..]
            .iter()
            .all(|slice| slice.member_ids.len() == 64)
    );
    assert!(
        evaluation[1..]
            .windows(2)
            .all(|pair| pair[0].member_ids == pair[1].member_ids)
    );
    let blind = manifest
        .slices
        .iter()
        .find(|slice| slice.split == ProtectedSplit::FinalBlindTest)
        .unwrap();
    assert!(blind.member_ids.is_empty());
    assert!(blind.hidden && !blind.supports_real_world_generalization);
    assert!(
        blind
            .limitation_codes
            .contains(&"no_independently_hidden_artifact".into())
    );
    assert!(!manifest.contract_plumbing_ready);
    assert!(!manifest.model_qualification_ready);
    assert_eq!(
        manifest
            .members
            .iter()
            .filter(|member| member.manufacturer_key.as_deref() == Some("adafruit"))
            .count(),
        9
    );
    assert!(manifest.inputs.iter().any(|input| {
        input.role == "reviewed_adafruit_tps61023_examples"
            && input.content_sha256.0
                == "5fd8d99a976f461b823dc022c59e7f24a6d694fb8956a2eb549067aadf960ab1"
            && input.row_count == 9
    }));
}

#[test]
fn v2_schema_snapshot_is_current_while_v1_remains_readable() {
    let generated = serde_json::to_value(schemars::schema_for!(ProtectedCorpusManifestV2)).unwrap();
    let checked: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/fdx4/protected-corpus-manifest-v2.schema.json"
    ))
    .unwrap();
    assert_eq!(generated, checked);
    let v1: datasheet_core::corpus::ProtectedCorpusManifest = serde_json::from_str(include_str!(
        "../../../evidence/fdx4-protected-corpus-manifest.json"
    ))
    .unwrap();
    v1.validate().unwrap();
}

#[test]
fn evaluation_tags_can_overlap_and_replay_deterministically() {
    let frozen = freeze_protected_corpus_v2(fixture()).unwrap();
    let held_out = digest('5');
    assert_eq!(
        frozen
            .slices
            .iter()
            .filter(|s| s.member_ids.contains(&held_out))
            .count(),
        4
    );
    let mut shuffled = frozen.clone();
    shuffled.members.reverse();
    shuffled.slices.reverse();
    for slice in &mut shuffled.slices {
        slice.member_ids.reverse();
        slice.held_out_from_train_by.reverse();
    }
    assert_eq!(freeze_protected_corpus_v2(shuffled).unwrap(), frozen);
}

#[test]
fn each_declared_group_is_checked_only_against_train() {
    for group in [
        GroupingLevel::ExactContent,
        GroupingLevel::NearDuplicate,
        GroupingLevel::Revision,
        GroupingLevel::SyntheticRoot,
        GroupingLevel::DocumentFamily,
        GroupingLevel::TemplateFamily,
        GroupingLevel::Manufacturer,
        GroupingLevel::PublisherLineage,
    ] {
        let mut manifest = fixture();
        let training = manifest.members[0].clone();
        let held_out = &mut manifest.members[1];
        match group {
            GroupingLevel::ExactContent => {
                held_out.example_content_sha256 = training.example_content_sha256
            }
            GroupingLevel::NearDuplicate => {
                held_out.near_duplicate_group_id = training.near_duplicate_group_id
            }
            GroupingLevel::Revision => held_out.revision_group_id = training.revision_group_id,
            GroupingLevel::SyntheticRoot => {
                held_out.evidence_class = CorpusEvidenceClass::SyntheticDerivative;
                held_out.real_world_generalization_eligible = false;
                held_out.synthetic_root_example_id = Some(training.example_id);
            }
            GroupingLevel::DocumentFamily => {
                held_out.document_family_id = training.document_family_id
            }
            GroupingLevel::TemplateFamily => {
                held_out.template_family_id = training.template_family_id
            }
            GroupingLevel::Manufacturer => held_out.manufacturer_key = training.manufacturer_key,
            GroupingLevel::PublisherLineage => {
                held_out.lineage_group_ids = training.lineage_group_ids
            }
        }
        for slice in &mut manifest.slices {
            if slice.split != ProtectedSplit::Train
                && slice.member_ids.contains(&held_out.example_id)
                && !slice.held_out_from_train_by.contains(&group)
            {
                slice.held_out_from_train_by.push(group);
            }
        }
        assert!(
            freeze_protected_corpus_v2(manifest).is_err(),
            "{group:?} leaked"
        );
    }
}

#[test]
fn synthetic_data_cannot_support_generalization_or_model_qualification() {
    let mut manifest = fixture();
    let blind = manifest
        .slices
        .iter_mut()
        .find(|s| s.split == ProtectedSplit::FinalBlindTest)
        .unwrap();
    blind.supports_real_world_generalization = true;
    assert!(freeze_protected_corpus_v2(manifest).is_err());
}

#[test]
fn manufacturer_is_enforced_only_when_the_slice_declares_it() {
    let mut manifest = fixture();
    manifest.members[1].manufacturer_key = manifest.members[0].manufacturer_key.clone();
    let development = manifest
        .slices
        .iter_mut()
        .find(|s| s.split == ProtectedSplit::Development)
        .unwrap();
    development.member_ids.clear();
    development
        .member_ids
        .push(manifest.members[2].example_id.clone());
    development
        .limitation_codes
        .push("synthetic_contract_only".into());
    development.supports_real_world_generalization = false;
    // The manufacturer-held-out declaration still catches this overlap.
    assert!(freeze_protected_corpus_v2(manifest).is_err());
}

#[test]
fn declared_grouping_requires_metadata_on_train_and_evaluation() {
    for missing in [
        "train-template",
        "evaluation-manufacturer",
        "train-revision",
    ] {
        let mut manifest = fixture();
        match missing {
            "train-template" => manifest.members[0].template_family_id = None,
            "evaluation-manufacturer" => manifest.members[1].manufacturer_key = None,
            "train-revision" => manifest.members[0].revision_group_id = None,
            _ => unreachable!(),
        }
        assert!(freeze_protected_corpus_v2(manifest).is_err(), "{missing}");
    }
}
