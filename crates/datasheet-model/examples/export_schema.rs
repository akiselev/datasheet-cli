// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_model::ModelManifest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas/fdx1/model-manifest-v1.schema.json"));
    let mut bytes = serde_json::to_vec_pretty(&schemars::schema_for!(ModelManifest))?;
    bytes.push(b'\n');
    fs::write(output, bytes)?;
    Ok(())
}
