// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_compiler::supervision::frozen_labeler_catalog;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tests/goldens/fdx3_labeler_catalog.json"));
    let mut bytes = serde_json::to_vec_pretty(&frozen_labeler_catalog()?)?;
    bytes.push(b'\n');
    fs::write(output, bytes)?;
    Ok(())
}
