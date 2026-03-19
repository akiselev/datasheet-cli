// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

//! Extract footprint drawings from PDF datasheets as cropped PNG images.
//!
//! Flow:
//! 1. Send PDF to Gemini (auto-splitting large PDFs) → bounding boxes
//! 2. Render only the needed pages via mupdf, crop, save as PNG

use crate::llm::{LlmProvider, resolve_api_key};
use crate::page_render;
use anyhow::{Result, anyhow};
use clap::Args;
use std::path::PathBuf;

const PROMPT: &str = include_str!("../prompts/extract-footprint-image.md");
const __DEFAULT__: &str = "__DEFAULT__";

#[derive(Args, Debug)]
pub struct FootprintImageArgs {
    /// Input PDF path
    pub pdf: PathBuf,

    /// Output directory for extracted footprint PNGs
    #[arg(long, short = 'o', default_value = ".")]
    pub out_dir: PathBuf,

    /// Render DPI (higher = better quality, larger files)
    #[arg(long, default_value = "300")]
    pub dpi: u32,

    /// Padding around the detected bounding box (0-1000 normalized units)
    #[arg(long, default_value = "20")]
    pub padding: u32,

    /// LLM provider
    #[arg(long, default_value = "gemini", hide = true)]
    pub provider: LlmProvider,

    /// Model name
    #[arg(long, default_value = __DEFAULT__)]
    pub model: String,

    /// API key (falls back to GOOGLE_API_KEY or GEMINI_API_KEY)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Optional base URL override for Gemini API
    #[arg(long)]
    pub base_url: Option<String>,

    /// Disable file caching (re-upload PDF every request)
    #[arg(long)]
    pub no_cache: bool,

    /// Save the whole page instead of cropping to the bounding box
    #[arg(long)]
    pub whole_page: bool,
}

pub fn run(args: &FootprintImageArgs) -> Result<()> {
    if !args.pdf.exists() {
        return Err(anyhow!("PDF not found: {}", args.pdf.display()));
    }
    std::fs::create_dir_all(&args.out_dir)
        .map_err(|e| anyhow!("creating output directory {}: {}", args.out_dir.display(), e))?;

    let api_key = resolve_api_key(args.provider, args.api_key.clone())?;
    let model = if args.model == __DEFAULT__ {
        "gemini-3.1-pro-preview".to_string()
    } else {
        args.model.clone()
    };

    // ── Step 1: Detect footprint locations (auto-splits large PDFs) ────
    let all_footprints = page_render::detect_pages(
        &args.pdf,
        PROMPT,
        args.no_cache,
        &api_key,
        &args.base_url,
        args.provider,
        &model,
        "FOOTPRINT-IMAGE",
    )?;

    if all_footprints.is_empty() {
        eprintln!("[FOOTPRINT-IMAGE] No footprint drawings detected in the PDF.");
        return Ok(());
    }

    eprintln!(
        "[FOOTPRINT-IMAGE] Found {} footprint(s):",
        all_footprints.len()
    );
    for fp in &all_footprints {
        eprintln!(
            "  page {}: {} (bbox: [{},{} → {},{}])",
            fp.page, fp.label, fp.bbox_x_min, fp.bbox_y_min, fp.bbox_x_max, fp.bbox_y_max
        );
    }

    // ── Step 2: Render pages and crop footprints to PNGs ───────────────
    let pdf_stem = args
        .pdf
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (saved, _) = page_render::render_and_crop(
        &args.pdf,
        &all_footprints,
        args.padding,
        args.dpi,
        &args.out_dir,
        &pdf_stem,
        args.whole_page,
        "FOOTPRINT-IMAGE",
    )?;

    eprintln!("[FOOTPRINT-IMAGE] Done — {} PNG(s) extracted.", saved);
    Ok(())
}
