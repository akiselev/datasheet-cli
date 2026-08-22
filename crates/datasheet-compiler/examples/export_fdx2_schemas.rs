// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_compiler::baseline::DeterministicBaseline;
use datasheet_compiler::evaluation::BaselineEvaluation;
use datasheet_compiler::ferrodoc::FerrodocConformanceInput;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas/fdx2"));
    fs::create_dir_all(&output)?;
    write_schema::<DeterministicBaseline>(&output, "deterministic-baseline-v1.schema.json")?;
    write_schema::<BaselineEvaluation>(&output, "evaluation-v1.schema.json")?;
    write_schema::<FerrodocConformanceInput>(&output, "ferrodoc-conformance-input-v1.schema.json")?;
    Ok(())
}

fn write_schema<T: schemars::JsonSchema>(
    output: &std::path::Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = schemars::schema_for!(T);
    let mut bytes = serde_json::to_vec_pretty(&schema)?;
    bytes.push(b'\n');
    fs::write(output.join(name), bytes)?;
    Ok(())
}
