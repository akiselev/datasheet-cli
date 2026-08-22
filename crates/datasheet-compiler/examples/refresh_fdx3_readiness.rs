// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use datasheet_core::supervision::ProductionReadinessManifest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../evidence/fdx3-production-readiness-2026-08-22.json")
        });
    let mut manifest: ProductionReadinessManifest = serde_json::from_slice(&fs::read(&path)?)?;
    manifest.manifest_digest = manifest.recompute_digest();
    manifest.validate()?;
    let mut bytes = serde_json::to_vec_pretty(&manifest)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}
