use datasheet_compiler::corpus::freeze_protected_corpus;
use datasheet_core::Sha256Digest;
use datasheet_core::corpus::{
    CorpusInputBinding, CorpusMember, ModelQualificationExhaustion, PROTECTED_CORPUS_SCHEMA,
    ProtectedCorpusManifest, ProtectedSplit, ProtectedSplitManifest,
};

fn digest(byte: char) -> Sha256Digest {
    Sha256Digest(std::iter::repeat_n(byte, 64).collect())
}

fn production() -> ProtectedCorpusManifest {
    serde_json::from_str(include_str!(
        "../../../evidence/fdx4-protected-corpus-manifest.json"
    ))
    .unwrap()
}

fn exhaustion() -> ModelQualificationExhaustion {
    serde_json::from_str(include_str!(
        "../../../evidence/fdx4-model-qualification-exhaustion-2026-08-22.json"
    ))
    .unwrap()
}

fn member(id: char, split: ProtectedSplit) -> CorpusMember {
    CorpusMember {
        example_id: digest(id),
        example_content_sha256: digest(char::from_u32(u32::from(id) + 1).unwrap()),
        assigned_split: split,
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

fn fixture(left: CorpusMember, right: CorpusMember) -> ProtectedCorpusManifest {
    let mut splits = ProtectedSplit::ALL
        .into_iter()
        .map(|split| ProtectedSplitManifest {
            split,
            member_ids: vec![],
            hidden: split == ProtectedSplit::FinalBlindTest,
            artifact_digest: digest('0'),
            limitation_codes: vec!["synthetic_minimized_empty_split".into()],
        })
        .collect::<Vec<_>>();
    for member in [&left, &right] {
        let split = splits
            .iter_mut()
            .find(|item| item.split == member.assigned_split)
            .unwrap();
        split.member_ids.push(member.example_id.clone());
        split.limitation_codes.clear();
    }
    ProtectedCorpusManifest {
        schema_version: PROTECTED_CORPUS_SCHEMA.into(),
        corpus_digest: digest('0'),
        datasheet_revision: "synthetic".into(),
        foundry_revision: "synthetic".into(),
        ontology_snapshot_digest: digest('1'),
        calibration_snapshot_digest: digest('2'),
        builder_configuration_digest: digest('3'),
        inputs: vec![CorpusInputBinding {
            role: "synthetic".into(),
            content_sha256: digest('4'),
            logical_id: None,
            row_count: 2,
        }],
        members: vec![left, right],
        exclusions: vec![],
        splits,
        model_qualification_ready: false,
        limitation_codes: vec!["synthetic_minimized_fixture".into()],
    }
}

#[test]
fn production_manifest_is_train_only_and_replays_canonically() {
    let manifest = production();
    manifest.validate().unwrap();
    assert_eq!(manifest.members.len(), 210);
    assert!(
        manifest
            .members
            .iter()
            .all(|member| member.assigned_split == ProtectedSplit::Train)
    );
    assert!(!manifest.model_qualification_ready);
    assert_eq!(manifest.splits.len(), 6);
    assert!(
        manifest
            .splits
            .iter()
            .filter(|split| split.split != ProtectedSplit::Train)
            .all(|split| split.member_ids.is_empty() && !split.limitation_codes.is_empty())
    );
    let mut shuffled = manifest.clone();
    shuffled.members.reverse();
    shuffled.splits.reverse();
    shuffled.inputs.reverse();
    assert_eq!(freeze_protected_corpus(shuffled).unwrap(), manifest);
}

#[test]
fn checked_in_qualification_exhaustion_is_bound_and_fails_closed() {
    let mut ledger = exhaustion();
    assert_eq!(ledger.qualification_digest, ledger.recompute_digest());
    ledger.validate().unwrap();
    assert_eq!(ledger.qualification_eligible_member_count, 0);
    assert_eq!(ledger.minimum_pairwise_disjoint_manufacturer_groups, 6);
    assert!(!ledger.model_qualification_ready);
    assert!(!ledger.fdx5_training_started);

    ledger.model_qualification_ready = true;
    ledger.qualification_digest = ledger.recompute_digest();
    assert!(ledger.validate().is_err());

    let mut ledger = exhaustion();
    ledger.fdx5_training_started = true;
    ledger.qualification_digest = ledger.recompute_digest();
    assert!(ledger.validate().is_err());

    let mut ledger = exhaustion();
    ledger.eligible_new_manufacturer_groups = 1;
    ledger.qualification_digest = ledger.recompute_digest();
    assert!(ledger.validate().is_err());
}

#[test]
fn checked_in_fdx4_schemas_match_rust_contracts() {
    fn check<T: schemars::JsonSchema>(bytes: &str) {
        let expected: serde_json::Value = serde_json::from_str(bytes).unwrap();
        assert_eq!(
            serde_json::to_value(schemars::schema_for!(T)).unwrap(),
            expected
        );
    }
    check::<ProtectedCorpusManifest>(include_str!(
        "../../../schemas/fdx4/protected-corpus-manifest-v1.schema.json"
    ));
    check::<ModelQualificationExhaustion>(include_str!(
        "../../../schemas/fdx4/model-qualification-exhaustion-v1.schema.json"
    ));
}

#[test]
fn every_related_group_kind_is_protected_against_cross_split_leakage() {
    for kind in [
        "content",
        "document",
        "template",
        "manufacturer",
        "lineage",
        "near",
        "revision",
        "synthetic",
    ] {
        let mut left = member('5', ProtectedSplit::Train);
        let mut right = member('7', ProtectedSplit::Development);
        match kind {
            "content" => right.example_content_sha256 = left.example_content_sha256.clone(),
            "document" => right.document_family_id = left.document_family_id.clone(),
            "template" => right.template_family_id = left.template_family_id.clone(),
            "manufacturer" => right.manufacturer_key = left.manufacturer_key.clone(),
            "lineage" => right.lineage_group_ids = left.lineage_group_ids.clone(),
            "near" => right.near_duplicate_group_id = left.near_duplicate_group_id.clone(),
            "revision" => right.revision_group_id = left.revision_group_id.clone(),
            "synthetic" => {
                let root = left.example_id.clone();
                left.synthetic_root_example_id = Some(root.clone());
                right.synthetic_root_example_id = Some(root);
            }
            _ => unreachable!(),
        }
        assert!(
            freeze_protected_corpus(fixture(left, right)).is_err(),
            "{kind} leaked"
        );
    }
}

#[test]
fn grouping_digests_and_input_cardinality_fail_closed() {
    let mut manifest = production();
    manifest.members[0].lineage_group_ids[0] = Sha256Digest("not-a-digest".into());
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());

    let mut manifest = production();
    manifest.members[0].revision_group_id = Some(Sha256Digest("not-a-digest".into()));
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());

    let mut manifest = production();
    manifest.inputs[0].row_count -= 1;
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());
}

#[test]
fn incomplete_grouping_can_never_claim_model_qualification() {
    let mut members = ProtectedSplit::ALL
        .into_iter()
        .zip(['0', '1', '2', '3', '4', '5'])
        .map(|(split, id)| member(id, split))
        .collect::<Vec<_>>();
    members[0].manufacturer_key = None;
    let splits = members
        .iter()
        .map(|member| ProtectedSplitManifest {
            split: member.assigned_split,
            member_ids: vec![member.example_id.clone()],
            hidden: member.assigned_split == ProtectedSplit::FinalBlindTest,
            artifact_digest: digest('0'),
            limitation_codes: vec![],
        })
        .collect::<Vec<_>>();
    let mut manifest = ProtectedCorpusManifest {
        schema_version: PROTECTED_CORPUS_SCHEMA.into(),
        corpus_digest: digest('0'),
        datasheet_revision: "synthetic".into(),
        foundry_revision: "synthetic".into(),
        ontology_snapshot_digest: digest('1'),
        calibration_snapshot_digest: digest('2'),
        builder_configuration_digest: digest('3'),
        inputs: vec![CorpusInputBinding {
            role: "synthetic".into(),
            content_sha256: digest('4'),
            logical_id: None,
            row_count: 6,
        }],
        members,
        exclusions: vec![],
        splits,
        model_qualification_ready: true,
        limitation_codes: vec![],
    };
    for split in &mut manifest.splits {
        split.artifact_digest = split.recompute_digest();
    }
    manifest.model_qualification_ready = true;
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());
}

#[test]
fn rights_restrictions_exclusions_hidden_state_and_split_completeness_fail_closed() {
    let mut manifest = production();
    manifest.members[0].training_permitted = false;
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());

    let mut manifest = production();
    manifest.members[0]
        .restriction_codes
        .push("restricted".into());
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());

    let mut manifest = production();
    manifest
        .exclusions
        .push(datasheet_core::corpus::CorpusExclusion {
            example_id: manifest.members[0].example_id.clone(),
            reason_codes: vec!["denied".into()],
            rights_snapshot_ids: vec![digest('f')],
        });
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());

    let mut manifest = production();
    manifest.splits.pop();
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());

    let mut manifest = production();
    let blind = manifest
        .splits
        .iter_mut()
        .find(|split| split.split == ProtectedSplit::FinalBlindTest)
        .unwrap();
    blind.hidden = false;
    manifest.corpus_digest = manifest.recompute_digest();
    assert!(manifest.validate().is_err());
}
