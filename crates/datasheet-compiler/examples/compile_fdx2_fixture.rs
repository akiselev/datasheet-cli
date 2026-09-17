// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_compiler::baseline::{Fdx2Document, compile_analog_power_baseline};
use datasheet_compiler::evaluation::{BaselineTruth, evaluate_baseline};
use datasheet_compiler::quantity::FDX2_REGISTRY_ID;
use datasheet_core::{PredicateVocabularySnapshot, ProducerIdentity, Sha256Digest};

/// Producer version the FDX2 baseline was frozen at.
///
/// Deliberately not `CARGO_PKG_VERSION`: the golden is a frozen artifact that
/// pins compiler behaviour, so a release version bump must not change its bytes.
const FROZEN_PRODUCER_VERSION: &str = "0.1.8";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let document: Fdx2Document = serde_json::from_slice(&fs::read(
        root.join("tests/fixtures/fdx2_synthetic_analog_power.json"),
    )?)?;
    let vocabulary: PredicateVocabularySnapshot = serde_json::from_slice(&fs::read(
        root.join("tests/fixtures/fdx2_predicate_vocabulary.json"),
    )?)?;
    let producer = ProducerIdentity {
        name: "datasheet-compiler".into(),
        version: FROZEN_PRODUCER_VERSION.into(),
        code_revision: "fdx2-frozen-baseline-v1".into(),
        configuration_digest: Sha256Digest(
            "4444444444444444444444444444444444444444444444444444444444444444".into(),
        ),
        document_ir_schema: document.schema_version.clone(),
        ontology_snapshot_digest: vocabulary.digest.clone(),
        quantitas_registry_identity: Some(FDX2_REGISTRY_ID.into()),
        model_digest: None,
    };
    let baseline =
        compile_analog_power_baseline(&document, producer, &vocabulary, Some("analog_power"))?;
    let mut bytes = serde_json::to_vec_pretty(&baseline)?;
    bytes.push(b'\n');
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let output = args.next();
    if let Some(output) = output {
        fs::write(output, bytes)?;
    } else {
        print!("{}", String::from_utf8(bytes)?);
    }
    if let Some(metrics_output) = args.next() {
        let truth: BaselineTruth = serde_json::from_slice(&fs::read(
            root.join("tests/fixtures/fdx2_synthetic_truth.json"),
        )?)?;
        let metrics = evaluate_baseline(&document, &baseline, &truth);
        let mut bytes = serde_json::to_vec_pretty(&metrics)?;
        bytes.push(b'\n');
        fs::write(metrics_output, bytes)?;
    }
    Ok(())
}
