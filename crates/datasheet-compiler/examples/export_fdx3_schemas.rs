// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_compiler::supervision::{CombinationResult, LabelerCatalogArtifact};
use datasheet_core::supervision::{
    CalibrationSnapshot, LabelVote, ProductionReadinessManifest, ReviewedDevelopmentSet,
    SourceVocabularyMap, StructuredFactRecord, TrainingExample, TrainingLabel,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas/fdx3"));
    fs::create_dir_all(&output)?;
    write_schema::<SourceVocabularyMap>(&output, "source-vocabulary-map-v1.schema.json")?;
    write_schema::<LabelerCatalogArtifact>(&output, "labeler-catalog-v1.schema.json")?;
    write_schema::<LabelVote>(&output, "silver-label-votes-v1.schema.json")?;
    write_schema::<TrainingLabel>(&output, "silver-training-labels-v1.schema.json")?;
    write_schema::<StructuredFactRecord>(&output, "silver-structured-facts-v1.schema.json")?;
    write_schema::<TrainingExample>(&output, "training-examples-v1.schema.json")?;
    write_schema::<ReviewedDevelopmentSet>(&output, "reviewed-development-set-v1.schema.json")?;
    write_schema::<CalibrationSnapshot>(&output, "labeler-calibration-v1.schema.json")?;
    write_schema::<CombinationResult>(&output, "combination-result-v1.schema.json")?;
    write_schema::<ProductionReadinessManifest>(&output, "production-readiness-v1.schema.json")?;
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
