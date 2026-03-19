// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

//! Extract specific pages/regions from a PDF using LLM-guided detection.
//!
//! Unlike footprint-image (which uses a hardcoded footprint-detection prompt),
//! this command accepts a user-supplied description of what to find.

use crate::llm::{LlmProvider, resolve_api_key};
use crate::page_render;
use anyhow::{Result, anyhow};
use clap::Args;
use std::path::PathBuf;

const PROMPT_TEMPLATE: &str = include_str!("../prompts/extract-pages.md");
const __DEFAULT__: &str = "__DEFAULT__";

#[derive(Args, Debug)]
pub struct ExtractPagesArgs {
    /// Input PDF path
    pub pdf: PathBuf,

    /// Description of what pages/regions to find (e.g. "pin configuration tables")
    #[arg(long, short = 'd')]
    pub description: Option<String>,

    /// File containing descriptions, one per line (sent as single LLM request)
    #[arg(long, short = 'b')]
    pub batch: Option<PathBuf>,

    /// Output directory for extracted PNGs
    #[arg(long, short = 'o', default_value = ".")]
    pub out_dir: PathBuf,

    /// Render DPI
    #[arg(long, default_value = "300")]
    pub dpi: u32,

    /// Padding around bounding box (0-1000 normalized)
    #[arg(long, default_value = "20")]
    pub padding: u32,

    /// LLM provider
    #[arg(long, default_value = "gemini", hide = true)]
    pub provider: LlmProvider,

    /// Model name
    #[arg(long, default_value = __DEFAULT__)]
    pub model: String,

    /// API key
    #[arg(long)]
    pub api_key: Option<String>,

    /// Base URL override
    #[arg(long)]
    pub base_url: Option<String>,

    /// Disable file caching
    #[arg(long)]
    pub no_cache: bool,

    /// Crop to bounding box instead of extracting whole pages
    #[arg(long)]
    pub bounded: bool,
}

pub fn run(args: &ExtractPagesArgs) -> Result<()> {
    if !args.pdf.exists() {
        return Err(anyhow!("PDF not found: {}", args.pdf.display()));
    }

    // Validate that exactly one of --description or --batch is provided
    let descriptions = match (&args.description, &args.batch) {
        (Some(_), Some(_)) => {
            return Err(anyhow!(
                "provide either --description or --batch, not both"
            ));
        }
        (None, None) => {
            return Err(anyhow!(
                "one of --description or --batch is required"
            ));
        }
        (Some(desc), None) => desc.clone(),
        (None, Some(batch_path)) => {
            std::fs::read_to_string(batch_path)
                .map_err(|e| anyhow!("reading batch file {}: {}", batch_path.display(), e))?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        }
    };

    std::fs::create_dir_all(&args.out_dir)
        .map_err(|e| anyhow!("creating output directory {}: {}", args.out_dir.display(), e))?;

    let api_key = resolve_api_key(args.provider, args.api_key.clone())?;
    let model = if args.model == __DEFAULT__ {
        "gemini-3.1-pro-preview".to_string()
    } else {
        args.model.clone()
    };

    let prompt = PROMPT_TEMPLATE.replace("{DESCRIPTIONS}", &descriptions);

    // ── Step 1: Detect locations (auto-splits large PDFs) ─────────────
    let locations = page_render::detect_pages(
        &args.pdf,
        &prompt,
        args.no_cache,
        &api_key,
        &args.base_url,
        args.provider,
        &model,
        "EXTRACT-PAGES",
    )?;

    if locations.is_empty() {
        eprintln!("[EXTRACT-PAGES] No matching pages/regions detected in the PDF.");
        println!("{{\"extractions\":[]}}");
        return Ok(());
    }

    eprintln!("[EXTRACT-PAGES] Found {} match(es):", locations.len());
    for loc in &locations {
        eprintln!(
            "  page {}: {} (bbox: [{},{} → {},{}])",
            loc.page, loc.label, loc.bbox_x_min, loc.bbox_y_min, loc.bbox_x_max, loc.bbox_y_max
        );
    }

    // ── Step 2: Render and crop to PNGs ───────────────────────────────
    let pdf_stem = args
        .pdf
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (saved, outputs) = page_render::render_and_crop(
        &args.pdf,
        &locations,
        args.padding,
        args.dpi,
        &args.out_dir,
        &pdf_stem,
        !args.bounded,
        "EXTRACT-PAGES",
    )?;

    eprintln!("[EXTRACT-PAGES] Done — {} PNG(s) extracted.", saved);

    // ── Step 3: Print JSON manifest to stdout ─────────────────────────
    let mut extractions = Vec::new();
    for ((_label, file_path), page_loc) in outputs.iter().zip(locations.iter()) {
        let filename = file_path
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
            .unwrap_or_default();
        extractions.push(serde_json::json!({
            "page": page_loc.page,
            "label": page_loc.label,
            "file": filename,
        }));
    }

    let manifest = serde_json::json!({ "extractions": extractions });
    println!("{}", serde_json::to_string_pretty(&manifest)?);

    Ok(())
}
