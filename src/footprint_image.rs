// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

//! Extract footprint drawings from PDF datasheets as cropped PNG images.
//!
//! Flow:
//! 1. Send PDF to Gemini (auto-splitting large PDFs) → bounding boxes
//! 2. Render only the needed pages via mupdf, crop, save as PNG

use crate::file_cache::FileCache;
use crate::llm::{
    AttachmentSource, FileReference, LlmClient, LlmProvider, LlmRequest, build_client,
    resolve_api_key,
};
use crate::pdf_split;
use anyhow::{Context, Result, anyhow};
use clap::Args;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

#[derive(Deserialize, Debug)]
struct FootprintLocations {
    footprints: Vec<FootprintLocation>,
}

#[derive(Deserialize, Debug)]
struct FootprintLocation {
    page: u32,
    label: String,
    bbox_y_min: u32,
    bbox_x_min: u32,
    bbox_y_max: u32,
    bbox_x_max: u32,
}

pub fn run(args: &FootprintImageArgs) -> Result<()> {
    if !args.pdf.exists() {
        return Err(anyhow!("PDF not found: {}", args.pdf.display()));
    }
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating output directory: {}", args.out_dir.display()))?;

    let api_key = resolve_api_key(args.provider, args.api_key.clone())?;
    let model = if args.model == __DEFAULT__ {
        "gemini-3.1-pro-preview".to_string()
    } else {
        args.model.clone()
    };

    // ── Step 1: Detect footprint locations (auto-splits large PDFs) ────
    let all_footprints = detect_footprints(args, &api_key, &model)?;

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

    let saved = render_and_crop(
        &args.pdf,
        &all_footprints,
        args.padding,
        args.dpi,
        &args.out_dir,
        &pdf_stem,
        args.whole_page,
    )?;

    eprintln!("[FOOTPRINT-IMAGE] Done — {} PNG(s) extracted.", saved);
    Ok(())
}

// ── LLM detection (with auto-split) ───────────────────────────────────

fn detect_footprints(
    args: &FootprintImageArgs,
    api_key: &str,
    model: &str,
) -> Result<Vec<FootprintLocation>> {
    let split_result = pdf_split::split_if_needed(&args.pdf)?;

    let client = build_client(args.provider, api_key.to_string(), args.base_url.clone())?;

    if let Some(ref split) = split_result {
        let mut all = Vec::new();
        for (i, part) in split.parts.iter().enumerate() {
            eprintln!(
                "\n[SPLIT] Detecting footprints in part {}/{} (pages {}-{})...",
                i + 1,
                split.parts.len(),
                part.start_page,
                part.end_page
            );
            let attachment = make_attachment(&part.path, args.no_cache, api_key, &args.base_url)?;
            let mut footprints = call_detect(&*client, model, attachment)?;
            // Remap page numbers from split-relative to original PDF
            for fp in &mut footprints {
                fp.page += part.start_page - 1;
            }
            all.extend(footprints);
        }
        Ok(all)
    } else {
        let attachment = make_attachment(&args.pdf, args.no_cache, api_key, &args.base_url)?;
        call_detect(&*client, model, attachment)
    }
}

fn call_detect(
    client: &dyn LlmClient,
    model: &str,
    attachment: AttachmentSource,
) -> Result<Vec<FootprintLocation>> {
    eprintln!("[FOOTPRINT-IMAGE] Detecting footprint locations via LLM...");
    let response = client.generate_json(LlmRequest {
        model: model.to_string(),
        prompt: PROMPT.to_string(),
        schema: location_schema(),
        attachment,
        temperature: Some(0.2),
    })?;
    let locations: FootprintLocations = serde_json::from_value(response.json)
        .context("parsing footprint locations from LLM response")?;
    Ok(locations.footprints)
}

// ── Rendering via mupdf ───────────────────────────────────────────────

fn render_and_crop(
    pdf_path: &Path,
    footprints: &[FootprintLocation],
    padding: u32,
    dpi: u32,
    out_dir: &Path,
    pdf_stem: &str,
    whole_page: bool,
) -> Result<u32> {
    let doc = mupdf::document::Document::open(
        pdf_path
            .to_str()
            .ok_or_else(|| anyhow!("non-UTF8 path"))?,
    )
    .context("opening PDF with mupdf")?;

    let total_pages = doc.page_count().context("getting page count")? as u32;

    // Group footprints by page so we render each page only once
    let mut by_page: HashMap<u32, Vec<(usize, &FootprintLocation)>> = HashMap::new();
    for (i, fp) in footprints.iter().enumerate() {
        by_page.entry(fp.page).or_default().push((i, fp));
    }

    let scale = dpi as f32 / 72.0;
    let mut saved = 0u32;

    for (page_num, fps) in &by_page {
        if *page_num < 1 || *page_num > total_pages {
            eprintln!(
                "[FOOTPRINT-IMAGE] Warning: skipping page {} (PDF has {} pages)",
                page_num, total_pages
            );
            continue;
        }

        // Render this page once
        let page = doc
            .load_page((*page_num as i32) - 1)
            .with_context(|| format!("loading page {}", page_num))?;

        let ctm = mupdf::Matrix {
            a: scale,
            b: 0.0,
            c: 0.0,
            d: scale,
            e: 0.0,
            f: 0.0,
        };
        let pixmap = page
            .to_pixmap(&ctm, &mupdf::Colorspace::device_rgb(), 0.0, false)
            .with_context(|| format!("rendering page {}", page_num))?;

        let w = pixmap.width() as u32;
        let h = pixmap.height() as u32;
        let n = pixmap.n() as u32;
        let samples = pixmap.samples().to_vec();

        let img: image::DynamicImage = if n == 4 {
            image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(w, h, samples)
                    .ok_or_else(|| anyhow!("pixmap→RGBA conversion failed"))?,
            )
        } else {
            image::DynamicImage::ImageRgb8(
                image::RgbImage::from_raw(w, h, samples)
                    .ok_or_else(|| anyhow!("pixmap→RGB conversion failed"))?,
            )
        };

        if whole_page {
            // One PNG per page — no duplication
            let filename = format!("{}_p{}.png", pdf_stem, page_num);
            let out_path = out_dir.join(&filename);
            img.save(&out_path)
                .with_context(|| format!("saving {}", out_path.display()))?;
            eprintln!("[FOOTPRINT-IMAGE] Saved: {}", out_path.display());
            saved += 1;
        } else {
            // Crop each footprint on this page
            for (i, fp) in fps {
                let x1 = (fp.bbox_x_min.saturating_sub(padding) as f64 / 1000.0 * w as f64) as u32;
                let y1 = (fp.bbox_y_min.saturating_sub(padding) as f64 / 1000.0 * h as f64) as u32;
                let x2 =
                    ((fp.bbox_x_max + padding).min(1000) as f64 / 1000.0 * w as f64) as u32;
                let y2 =
                    ((fp.bbox_y_max + padding).min(1000) as f64 / 1000.0 * h as f64) as u32;
                let cw = x2.saturating_sub(x1).max(1);
                let ch = y2.saturating_sub(y1).max(1);
                let cropped = img.crop_imm(x1, y1, cw, ch);

                let safe_label = sanitize_label(&fp.label);
                let filename = format!("{}_{}_p{}_{}.png", pdf_stem, i + 1, fp.page, safe_label);
                let out_path = out_dir.join(&filename);
                cropped
                    .save(&out_path)
                    .with_context(|| format!("saving {}", out_path.display()))?;
                eprintln!("[FOOTPRINT-IMAGE] Saved: {}", out_path.display());
                saved += 1;
            }
        }
    }

    Ok(saved)
}

// ── Helpers ───────────────────────────────────────────────────────────

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn make_attachment(
    pdf: &Path,
    no_cache: bool,
    api_key: &str,
    base_url: &Option<String>,
) -> Result<AttachmentSource> {
    if no_cache {
        let data =
            std::fs::read(pdf).with_context(|| format!("reading {}", pdf.display()))?;
        Ok(AttachmentSource::Inline(crate::llm::Attachment {
            mime_type: "application/pdf".to_string(),
            data,
        }))
    } else {
        let mut cache = FileCache::new(api_key.to_string(), base_url.clone())
            .context("initializing file cache")?;
        let cached = cache
            .get_or_upload(pdf)
            .context("uploading PDF to Gemini")?;
        Ok(AttachmentSource::FileUri(FileReference {
            mime_type: "application/pdf".to_string(),
            file_uri: cached.uri,
        }))
    }
}

fn location_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "footprints": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "page": {
                            "type": "integer",
                            "description": "1-based page number containing the footprint drawing"
                        },
                        "label": {
                            "type": "string",
                            "description": "Description of the footprint, e.g. 'SOIC-8 Package Dimensions'"
                        },
                        "bbox_y_min": {
                            "type": "integer",
                            "description": "Top edge of bounding box (0-1000 normalized, 0 = top of page)"
                        },
                        "bbox_x_min": {
                            "type": "integer",
                            "description": "Left edge of bounding box (0-1000 normalized, 0 = left of page)"
                        },
                        "bbox_y_max": {
                            "type": "integer",
                            "description": "Bottom edge of bounding box (0-1000 normalized)"
                        },
                        "bbox_x_max": {
                            "type": "integer",
                            "description": "Right edge of bounding box (0-1000 normalized)"
                        }
                    },
                    "required": ["page", "label", "bbox_y_min", "bbox_x_min", "bbox_y_max", "bbox_x_max"]
                }
            }
        },
        "required": ["footprints"]
    })
}
