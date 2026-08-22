// SPDX-License-Identifier: MIT OR Apache-2.0
//! FDX1 manifest inspection shell. Training/export arrives in later gated phases.

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use datasheet_model::ModelManifest;

#[derive(Debug, Parser)]
#[command(about = "Inspect a versioned datasheet model manifest")]
struct Args {
    /// Manifest to validate and print in canonical pretty JSON.
    manifest: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let manifest: ModelManifest = serde_json::from_slice(&fs::read(args.manifest)?)?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
