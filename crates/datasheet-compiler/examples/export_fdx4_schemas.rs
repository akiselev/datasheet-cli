use std::{fs, path::Path};

use datasheet_core::corpus::{ModelQualificationExhaustion, ProtectedCorpusManifest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/fdx4");
    fs::create_dir_all(&output)?;
    let schema = schemars::schema_for!(ProtectedCorpusManifest);
    let mut bytes = serde_json::to_vec_pretty(&schema)?;
    bytes.push(b'\n');
    fs::write(
        output.join("protected-corpus-manifest-v1.schema.json"),
        bytes,
    )?;
    let schema = schemars::schema_for!(ModelQualificationExhaustion);
    let mut bytes = serde_json::to_vec_pretty(&schema)?;
    bytes.push(b'\n');
    fs::write(
        output.join("model-qualification-exhaustion-v1.schema.json"),
        bytes,
    )?;
    Ok(())
}
