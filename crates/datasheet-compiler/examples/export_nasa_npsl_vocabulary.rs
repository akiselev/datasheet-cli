// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};

use datasheet_compiler::supervision::{SourceVocabularyMapInput, derive_source_vocabulary_map};
use datasheet_core::supervision::{SourceVocabularyMap, SupervisionProducer};
use datasheet_core::{
    ParsedQuantityShape, PredicateTerm, PredicateVocabularySnapshot, Sha256Digest,
};

fn term(
    id: &str,
    name: &str,
    description: &str,
    symbol: &str,
    kind: &str,
    dimensionality: &str,
    shape: ParsedQuantityShape,
) -> PredicateTerm {
    PredicateTerm {
        id: id.into(),
        canonical_name: name.into(),
        description: description.into(),
        aliases: Vec::new(),
        symbol_aliases: vec![symbol.into()],
        expected_quantity_kind: Some(kind.into()),
        expected_dimensionality: Some(dimensionality.into()),
        expected_shapes: vec![shape],
        regime_constraints: Vec::new(),
        category_scope: vec!["discrete_semiconductor".into(), "analog_power".into()],
    }
}

fn vocabulary() -> PredicateVocabularySnapshot {
    let mut terms = vec![
        term(
            "discrete.bjt.collector_current",
            "BJT collector current",
            "Collector current stated for a bipolar junction transistor.",
            "I_C",
            "si:ElectricCurrent",
            "A",
            ParsedQuantityShape::Scalar,
        ),
        term(
            "discrete.bjt.collector_base_voltage.open_emitter",
            "BJT collector-base voltage with emitter open",
            "Collector-base voltage V_CBO; the emitter-open condition is carried by the symbol and predicate identity.",
            "V_CBO",
            "electronics:ElectricPotential",
            "kg m^2 s^-3 A^-1",
            ParsedQuantityShape::Scalar,
        ),
        term(
            "discrete.bjt.collector_emitter_voltage.open_base",
            "BJT collector-emitter voltage with base open",
            "Collector-emitter voltage V_CEO; the base-open condition is carried by the symbol and predicate identity.",
            "V_CEO",
            "electronics:ElectricPotential",
            "kg m^2 s^-3 A^-1",
            ParsedQuantityShape::Scalar,
        ),
        term(
            "discrete.bjt.emitter_base_voltage.open_collector",
            "BJT emitter-base voltage with collector open",
            "Emitter-base voltage V_EBO; the collector-open condition is carried by the symbol and predicate identity.",
            "V_EBO",
            "electronics:ElectricPotential",
            "kg m^2 s^-3 A^-1",
            ParsedQuantityShape::Scalar,
        ),
        term(
            "discrete.bjt.forward_current_gain",
            "BJT forward current gain",
            "Forward common-emitter current gain h_FE represented as the source-stated range.",
            "h_FE",
            "datasheet:Ratio",
            "1",
            ParsedQuantityShape::Range,
        ),
        term(
            "condition.bjt.collector_current.for_forward_gain",
            "Collector-current condition for BJT forward gain",
            "Collector current at which the source-stated h_FE range applies.",
            "I_C",
            "si:ElectricCurrent",
            "A",
            ParsedQuantityShape::Scalar,
        ),
    ];
    terms.sort_by(|left, right| left.id.cmp(&right.id));
    let digest = Sha256Digest::of_bytes(
        &serde_json::to_vec(&("predicate-vocabulary/1", &terms))
            .expect("vocabulary serialization cannot fail"),
    );
    PredicateVocabularySnapshot {
        schema_version: "predicate-vocabulary/1".into(),
        digest,
        terms,
    }
}

fn write_pretty(
    path: &Path,
    value: &impl serde::Serialize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("evidence"));
    let binding_bytes = fs::read(root.join("evidence/fdx3-nasa-npsl-source-binding.json"))?;
    let binding: serde_json::Value = serde_json::from_slice(&binding_bytes)?;
    let observation_ids = binding["mapping_observation_ids"]
        .as_object()
        .ok_or("mapping_observation_ids object")?;
    let producer = SupervisionProducer {
        name: "datasheet-fdx3-nasa-npsl-vocabulary".into(),
        version: "1".into(),
        build: None,
        configuration_digest: Sha256Digest::of_bytes(&binding_bytes),
        model_digest: None,
    };
    let vocabulary = vocabulary();
    let specs = [
        (
            "I_C",
            "discrete.bjt.collector_current",
            "si:ElectricCurrent",
            ParsedQuantityShape::Scalar,
        ),
        (
            "V_CBO",
            "discrete.bjt.collector_base_voltage.open_emitter",
            "electronics:ElectricPotential",
            ParsedQuantityShape::Scalar,
        ),
        (
            "V_CEO",
            "discrete.bjt.collector_emitter_voltage.open_base",
            "electronics:ElectricPotential",
            ParsedQuantityShape::Scalar,
        ),
        (
            "V_EBO",
            "discrete.bjt.emitter_base_voltage.open_collector",
            "electronics:ElectricPotential",
            ParsedQuantityShape::Scalar,
        ),
        (
            "h_FE_min_max",
            "discrete.bjt.forward_current_gain",
            "datasheet:Ratio",
            ParsedQuantityShape::Range,
        ),
        (
            "h_FE_at_I_C",
            "condition.bjt.collector_current.for_forward_gain",
            "si:ElectricCurrent",
            ParsedQuantityShape::Scalar,
        ),
    ];
    let mut maps = specs
        .into_iter()
        .map(|(field, predicate, kind, shape)| {
            derive_source_vocabulary_map(
                SourceVocabularyMapInput {
                    source_schema: "bronze.structured_fact_observations/v1".into(),
                    external_namespace: "NASA.NEPP.NPSL".into(),
                    external_field_key: field.into(),
                    predicate_term_id: predicate.into(),
                    expected_quantity_kind: Some(kind.into()),
                    expected_value_shapes: vec![shape],
                    regime_hint: None,
                    category_scope: vec!["analog_power".into(), "discrete_semiconductor".into()],
                    mapping_observation_ids: vec![Sha256Digest(
                        observation_ids[field]
                            .as_str()
                            .ok_or("mapping observation ID")?
                            .into(),
                    )],
                    producer: producer.clone(),
                    mapping_version: "npsl-lp-npt/1998-03+policy-2026-08-22".into(),
                },
                &vocabulary,
            )
            .map_err(Into::into)
        })
        .collect::<Result<Vec<SourceVocabularyMap>, Box<dyn std::error::Error>>>()?;
    maps.sort_by(|left, right| left.external_field_key.cmp(&right.external_field_key));
    write_pretty(
        &output.join("fdx3-nasa-npsl-predicate-vocabulary.json"),
        &vocabulary,
    )?;
    write_pretty(
        &output.join("fdx3-nasa-npsl-source-vocabulary-maps.json"),
        &maps,
    )?;
    Ok(())
}
