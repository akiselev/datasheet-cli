// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_compiler::ferrodoc::FerrodocConformanceInput;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/ferrodoc_fdx0_datasheet_input.json")
        });
    let fixture: FerrodocConformanceInput = serde_json::from_slice(&fs::read(input)?)?;
    println!("delta_id={}", fixture.evidence_delta.logical_id()?);
    println!("state_id={}", fixture.document_state_manifest.logical_id()?);
    Ok(())
}
