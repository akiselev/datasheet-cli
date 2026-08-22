// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use datasheet_compiler::baseline::{Fdx2Document, compile_analog_power_baseline};
use datasheet_compiler::quantity::FDX2_REGISTRY_ID;
use datasheet_compiler::{FixtureDocumentIr, sketch_from_fixture};
use datasheet_core::{PredicateVocabularySnapshot, ProducerIdentity, Sha256Digest};

#[derive(Args, Debug)]
pub struct SketchArgs {
    /// Minimized, synthetic FDX1 bridge fixture (not a production PDF/DocumentIR parser)
    pub input: PathBuf,

    /// Output path; stdout when omitted
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Exact source revision for deterministic producer identity
    #[arg(long)]
    pub code_revision: String,

    /// SHA-256 of the compiler configuration
    #[arg(long)]
    pub configuration_digest: String,

    /// SHA-256 of the predicate vocabulary snapshot
    #[arg(long)]
    pub ontology_snapshot_digest: String,

    /// Quantitas registry identity used by normalized fixture quantities
    #[arg(long)]
    pub quantitas_registry_identity: Option<String>,
}

pub fn run_sketch(args: &SketchArgs) -> Result<()> {
    let input = fs::read(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let fixture: FixtureDocumentIr = serde_json::from_slice(&input)
        .with_context(|| format!("invalid FDX1 fixture {}", args.input.display()))?;
    let producer = ProducerIdentity {
        name: "datasheet-compiler".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        code_revision: args.code_revision.clone(),
        configuration_digest: Sha256Digest(args.configuration_digest.clone()),
        document_ir_schema: fixture.schema_version.clone(),
        ontology_snapshot_digest: Sha256Digest(args.ontology_snapshot_digest.clone()),
        quantitas_registry_identity: args.quantitas_registry_identity.clone(),
        model_digest: None,
    };
    let sketch = sketch_from_fixture(&fixture, producer).context("fixture did not compile")?;
    let mut encoded = serde_json::to_vec_pretty(&sketch)?;
    encoded.push(b'\n');
    if let Some(output) = &args.output {
        fs::write(output, encoded)
            .with_context(|| format!("failed to write {}", output.display()))?;
    } else {
        print!("{}", String::from_utf8(encoded)?);
    }
    Ok(())
}

#[derive(Args, Debug)]
pub struct BaselineArgs {
    /// Minimized FDX0-shaped DocumentIR JSON (not a PDF parser)
    pub input: PathBuf,
    /// Frozen predicate vocabulary snapshot JSON
    pub vocabulary: PathBuf,
    /// Output path; stdout when omitted
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Exact source revision for deterministic producer identity
    #[arg(long)]
    pub code_revision: String,
    /// SHA-256 of the deterministic compiler configuration
    #[arg(long)]
    pub configuration_digest: String,
    /// Optional category prior used only as an auditable ranking feature
    #[arg(long, default_value = "analog_power")]
    pub category_prior: String,
}

pub fn run_baseline(args: &BaselineArgs) -> Result<()> {
    let input = fs::read(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let document: Fdx2Document = serde_json::from_slice(&input)
        .with_context(|| format!("invalid FDX2 document {}", args.input.display()))?;
    let vocabulary_bytes = fs::read(&args.vocabulary)
        .with_context(|| format!("failed to read {}", args.vocabulary.display()))?;
    let vocabulary: PredicateVocabularySnapshot = serde_json::from_slice(&vocabulary_bytes)
        .with_context(|| format!("invalid vocabulary {}", args.vocabulary.display()))?;
    let producer = ProducerIdentity {
        name: "datasheet-compiler".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        code_revision: args.code_revision.clone(),
        configuration_digest: Sha256Digest(args.configuration_digest.clone()),
        document_ir_schema: document.schema_version.clone(),
        ontology_snapshot_digest: vocabulary.digest.clone(),
        quantitas_registry_identity: Some(FDX2_REGISTRY_ID.into()),
        model_digest: None,
    };
    let baseline =
        compile_analog_power_baseline(&document, producer, &vocabulary, Some(&args.category_prior))
            .context("deterministic analog/power baseline refused the input")?;
    let mut encoded = serde_json::to_vec_pretty(&baseline)?;
    encoded.push(b'\n');
    if let Some(output) = &args.output {
        fs::write(output, encoded)
            .with_context(|| format!("failed to write {}", output.display()))?;
    } else {
        print!("{}", String::from_utf8(encoded)?);
    }
    Ok(())
}
