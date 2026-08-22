// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic, missing-is-wrong evaluation for the frozen FDX2 baseline.

use std::collections::{BTreeMap, BTreeSet};

use datasheet_core::{GroundingStatus, ParsedQuantityShape, SpecificationRegime, ValueRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::baseline::{DeterministicBaseline, Fdx2Document, InferenceStatus, RowExtractionStatus};

pub const FDX2_EVALUATION_SCHEMA: &str = "datasheet-fdx2-evaluation/1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SectionTruth {
    pub heading: String,
    pub regime: SpecificationRegime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HeaderTruth {
    pub signature: String,
    pub roles: Vec<crate::baseline::ColumnRole>,
    pub abstained: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RowTruth {
    pub parameter_text: String,
    pub regime: SpecificationRegime,
    pub value_role: ValueRole,
    pub raw_quantity: String,
    pub parsed_shape: ParsedQuantityShape,
    pub canonical_value: Option<String>,
    pub predicate_term_id: Option<String>,
    pub condition_texts: Vec<String>,
    pub accepted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineTruth {
    pub schema_version: String,
    pub sections: Vec<SectionTruth>,
    pub headers: Vec<HeaderTruth>,
    pub rows: Vec<RowTruth>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RatioMetric {
    pub numerator: u32,
    pub denominator: u32,
    pub value: f64,
}

impl RatioMetric {
    fn new(numerator: usize, denominator: usize) -> Self {
        Self {
            numerator: numerator as u32,
            denominator: denominator as u32,
            value: if denominator == 0 {
                0.0
            } else {
                numerator as f64 / denominator as f64
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResourceObservation {
    pub wall_time_milliseconds: Option<u64>,
    pub peak_resident_bytes: Option<u64>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineEvaluation {
    pub schema_version: String,
    pub section_precision: RatioMetric,
    pub section_recall: RatioMetric,
    pub header_role_accuracy: RatioMetric,
    pub quantity_parse_accuracy: RatioMetric,
    pub predicate_top1_accuracy: RatioMetric,
    pub predicate_top5_accuracy: RatioMetric,
    pub condition_span_accuracy: RatioMetric,
    pub exact_claim_accuracy: RatioMetric,
    pub claim_anchor_validity: RatioMetric,
    pub ungrounded_output_count: u32,
    pub abstention_count: u32,
    pub coverage: RatioMetric,
    pub resources: ResourceObservation,
}

pub fn evaluate_baseline(
    document: &Fdx2Document,
    baseline: &DeterministicBaseline,
    truth: &BaselineTruth,
) -> BaselineEvaluation {
    let actual_sections: BTreeSet<_> = baseline
        .sketch
        .sections
        .iter()
        .flat_map(|section| {
            section
                .deterministic_regime_candidates
                .iter()
                .map(|regime| {
                    (
                        section.heading_text.clone().unwrap_or_default(),
                        regime.clone(),
                    )
                })
        })
        .collect();
    let expected_sections: BTreeSet<_> = truth
        .sections
        .iter()
        .map(|section| (section.heading.clone(), section.regime.clone()))
        .collect();
    let section_matches = actual_sections.intersection(&expected_sections).count();

    let actual_headers: BTreeMap<_, _> = baseline
        .header_inferences
        .iter()
        .map(|header| (header.signature.as_str(), header))
        .collect();
    let header_matches = truth
        .headers
        .iter()
        .filter(|expected| {
            actual_headers
                .get(expected.signature.as_str())
                .is_some_and(|actual| {
                    actual.roles == expected.roles
                        && (actual.status == InferenceStatus::Abstained) == expected.abstained
                })
        })
        .count();

    let quantities: BTreeMap<_, _> = baseline
        .sketch
        .quantities
        .iter()
        .map(|quantity| (&quantity.id, quantity))
        .collect();
    let conditions: BTreeMap<_, _> = baseline
        .sketch
        .conditions
        .iter()
        .map(|condition| (&condition.id, condition.raw_text.as_str()))
        .collect();
    let row_key = |parameter: &str, regime: &SpecificationRegime, role: &ValueRole| {
        (parameter.to_string(), regime.clone(), role.clone())
    };
    let actual_rows: BTreeMap<_, _> = baseline
        .row_results
        .iter()
        .map(|row| {
            (
                row_key(&row.parameter_text, &row.regime, &row.value_role),
                row,
            )
        })
        .collect();
    let mut quantity_matches = 0;
    let mut predicate_top1 = 0;
    let mut predicate_top5 = 0;
    let mut condition_matches = 0;
    let mut exact_claims = 0;
    for expected in &truth.rows {
        let Some(actual) = actual_rows.get(&row_key(
            &expected.parameter_text,
            &expected.regime,
            &expected.value_role,
        )) else {
            continue;
        };
        let quantity_matches_truth = actual
            .value_candidate_id
            .as_ref()
            .and_then(|id| quantities.get(id))
            .is_some_and(|quantity| {
                quantity.raw_text == expected.raw_quantity
                    && quantity.parsed_shape == expected.parsed_shape
                    && quantity
                        .normalized_quantity
                        .as_ref()
                        .map(|normalized| normalized.canonical_value.as_str())
                        == expected.canonical_value.as_deref()
            })
            || (actual.value_candidate_id.is_none()
                && expected.parsed_shape == ParsedQuantityShape::TextUnparsed
                && expected.canonical_value.is_none()
                && matches!(
                    expected.raw_quantity.trim(),
                    "" | "-" | "—" | "–" | "N/A" | "NA"
                ));
        quantity_matches += usize::from(quantity_matches_truth);
        let top1 = actual
            .predicate_candidates
            .first()
            .map(|candidate| candidate.predicate_term_id.as_str())
            == expected.predicate_term_id.as_deref();
        if expected.predicate_term_id.is_some() {
            predicate_top1 += usize::from(top1);
        }
        let top5 = expected
            .predicate_term_id
            .as_ref()
            .is_some_and(|expected_id| {
                actual
                    .predicate_candidates
                    .iter()
                    .any(|candidate| &candidate.predicate_term_id == expected_id)
            })
            || (expected.predicate_term_id.is_none() && actual.predicate_candidates.is_empty());
        if expected.predicate_term_id.is_some() {
            predicate_top5 += usize::from(top5);
        }
        let mut actual_condition_texts: Vec<_> = actual
            .condition_candidate_ids
            .iter()
            .filter_map(|id| conditions.get(id).copied())
            .collect();
        actual_condition_texts.sort_unstable();
        let mut expected_condition_texts: Vec<_> = expected
            .condition_texts
            .iter()
            .map(String::as_str)
            .collect();
        expected_condition_texts.sort_unstable();
        condition_matches += usize::from(actual_condition_texts == expected_condition_texts);
        let accepted = actual.status == RowExtractionStatus::AcceptedByExtractor;
        if expected.accepted {
            exact_claims += usize::from(
                quantity_matches_truth
                    && top1
                    && accepted
                    && actual_condition_texts == expected_condition_texts,
            );
        }
    }

    let valid_anchors = baseline
        .claim_bundle
        .claims
        .iter()
        .filter(|claim| {
            !claim.evidence_anchors.is_empty()
                && claim
                    .evidence_anchors
                    .iter()
                    .all(|anchor| document.validate_anchor(anchor).is_ok())
        })
        .count();
    let ungrounded_output_count = baseline
        .claim_bundle
        .claims
        .iter()
        .filter(|claim| claim.grounding_status != GroundingStatus::Grounded)
        .count();
    let accepted_rows = baseline
        .row_results
        .iter()
        .filter(|row| row.status == RowExtractionStatus::AcceptedByExtractor)
        .count();
    let abstentions = baseline
        .row_results
        .iter()
        .filter(|row| row.status != RowExtractionStatus::AcceptedByExtractor)
        .count()
        + baseline.abstained_tables as usize;

    BaselineEvaluation {
        schema_version: FDX2_EVALUATION_SCHEMA.into(),
        section_precision: RatioMetric::new(section_matches, actual_sections.len()),
        section_recall: RatioMetric::new(section_matches, expected_sections.len()),
        header_role_accuracy: RatioMetric::new(header_matches, truth.headers.len()),
        quantity_parse_accuracy: RatioMetric::new(quantity_matches, truth.rows.len()),
        predicate_top1_accuracy: RatioMetric::new(
            predicate_top1,
            truth
                .rows
                .iter()
                .filter(|row| row.predicate_term_id.is_some())
                .count(),
        ),
        predicate_top5_accuracy: RatioMetric::new(
            predicate_top5,
            truth
                .rows
                .iter()
                .filter(|row| row.predicate_term_id.is_some())
                .count(),
        ),
        condition_span_accuracy: RatioMetric::new(condition_matches, truth.rows.len()),
        exact_claim_accuracy: RatioMetric::new(
            exact_claims,
            truth.rows.iter().filter(|row| row.accepted).count(),
        ),
        claim_anchor_validity: RatioMetric::new(valid_anchors, baseline.claim_bundle.claims.len()),
        ungrounded_output_count: ungrounded_output_count as u32,
        abstention_count: abstentions as u32,
        coverage: RatioMetric::new(accepted_rows, baseline.row_results.len()),
        resources: ResourceObservation {
            wall_time_milliseconds: None,
            peak_resident_bytes: None,
            note: "not frozen: host-dependent observations belong in CI artifacts".into(),
        },
    }
}
