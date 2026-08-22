// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use datasheet_compiler::{FixtureDocumentIr, sketch_from_fixture};
use datasheet_core::{ProducerIdentity, Sha256Digest};

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
    let encoded = serde_json::to_vec_pretty(&sketch)?;
    if let Some(output) = &args.output {
        fs::write(output, encoded)
            .with_context(|| format!("failed to write {}", output.display()))?;
    } else {
        println!("{}", String::from_utf8(encoded)?);
    }
    Ok(())
}
