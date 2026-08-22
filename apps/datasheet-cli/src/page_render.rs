// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

//! Shared page rendering and LLM-guided detection used by footprint-image
//! and extract-pages commands.

use crate::file_cache::FileCache;
use crate::llm::{
    AttachmentSource, FileReference, LlmClient, LlmProvider, LlmRequest, build_client,
};
use crate::pdf_split;
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Shared types ──────────────────────────────────────────────────────

pub struct PageLocation {
    pub page: u32,
    pub label: String,
    pub bbox_x_min: u32,
    pub bbox_y_min: u32,
    pub bbox_x_max: u32,
    pub bbox_y_max: u32,
}

#[derive(Deserialize, Debug)]
struct PageLocations {
    results: Vec<PageLocationRaw>,
}

#[derive(Deserialize, Debug)]
struct PageLocationRaw {
    page: u32,
    label: String,
    bbox_y_min: u32,
    bbox_x_min: u32,
    bbox_y_max: u32,
    bbox_x_max: u32,
}

impl From<PageLocationRaw> for PageLocation {
    fn from(r: PageLocationRaw) -> Self {
        Self {
            page: r.page,
            label: r.label,
            bbox_x_min: r.bbox_x_min,
            bbox_y_min: r.bbox_y_min,
            bbox_x_max: r.bbox_x_max,
            bbox_y_max: r.bbox_y_max,
        }
    }
}

// ── JSON schema ───────────────────────────────────────────────────────

pub fn location_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "results": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "page": {
                            "type": "integer",
                            "description": "1-based page number"
                        },
                        "label": {
                            "type": "string",
                            "description": "Descriptive label for the found content"
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
        "required": ["results"]
    })
}

// ── LLM detection ─────────────────────────────────────────────────────

/// Detect pages/regions in a PDF using a user-supplied prompt.
///
/// Handles PDF splitting for large documents automatically.
pub fn detect_pages(
    pdf: &Path,
    prompt: &str,
    no_cache: bool,
    api_key: &str,
    base_url: &Option<String>,
    provider: LlmProvider,
    model: &str,
    log_prefix: &str,
) -> Result<Vec<PageLocation>> {
    let split_result = pdf_split::split_if_needed(pdf)?;

    let client = build_client(provider, api_key.to_string(), base_url.clone())?;

    if let Some(ref split) = split_result {
        let mut all = Vec::new();
        for (i, part) in split.parts.iter().enumerate() {
            eprintln!(
                "\n[SPLIT] Detecting in part {}/{} (pages {}-{})...",
                i + 1,
                split.parts.len(),
                part.start_page,
                part.end_page
            );
            let attachment = make_attachment(&part.path, no_cache, api_key, base_url)?;
            let mut locations = call_detect(&*client, model, prompt, attachment, log_prefix)?;
            for loc in &mut locations {
                loc.page += part.start_page - 1;
            }
            all.extend(locations);
        }
        Ok(all)
    } else {
        let attachment = make_attachment(pdf, no_cache, api_key, base_url)?;
        call_detect(&*client, model, prompt, attachment, log_prefix)
    }
}

fn call_detect(
    client: &dyn LlmClient,
    model: &str,
    prompt: &str,
    attachment: AttachmentSource,
    log_prefix: &str,
) -> Result<Vec<PageLocation>> {
    eprintln!("[{}] Detecting locations via LLM...", log_prefix);
    let response = client.generate_json(LlmRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        schema: location_schema(),
        attachment,
        temperature: Some(0.2),
    })?;
    let locations: PageLocations = serde_json::from_value(response.json)
        .context("parsing page locations from LLM response")?;
    Ok(locations.results.into_iter().map(PageLocation::from).collect())
}

// ── Rendering ─────────────────────────────────────────────────────────

/// Render and crop detected page regions to PNG files.
///
/// Returns the number of PNG files saved and a list of (label, filename) pairs.
pub fn render_and_crop(
    pdf_path: &Path,
    locations: &[PageLocation],
    padding: u32,
    dpi: u32,
    out_dir: &Path,
    pdf_stem: &str,
    whole_page: bool,
    log_prefix: &str,
) -> Result<(u32, Vec<(String, PathBuf)>)> {
    let doc = mupdf::document::Document::open(
        pdf_path
            .to_str()
            .ok_or_else(|| anyhow!("non-UTF8 path"))?,
    )
    .context("opening PDF with mupdf")?;

    let total_pages = doc.page_count().context("getting page count")? as u32;

    let mut by_page: HashMap<u32, Vec<(usize, &PageLocation)>> = HashMap::new();
    for (i, loc) in locations.iter().enumerate() {
        by_page.entry(loc.page).or_default().push((i, loc));
    }

    let scale = dpi as f32 / 72.0;
    let mut saved = 0u32;
    let mut outputs: Vec<(String, PathBuf)> = Vec::new();

    for (page_num, locs) in &by_page {
        if *page_num < 1 || *page_num > total_pages {
            eprintln!(
                "[{}] Warning: skipping page {} (PDF has {} pages)",
                log_prefix, page_num, total_pages
            );
            continue;
        }

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
            let filename = format!("{}_p{}.png", pdf_stem, page_num);
            let out_path = out_dir.join(&filename);
            img.save(&out_path)
                .with_context(|| format!("saving {}", out_path.display()))?;
            eprintln!("[{}] Saved: {}", log_prefix, out_path.display());
            outputs.push((format!("page {}", page_num), out_path));
            saved += 1;
        } else {
            for (i, loc) in locs {
                let x1 = (loc.bbox_x_min.saturating_sub(padding) as f64 / 1000.0 * w as f64) as u32;
                let y1 = (loc.bbox_y_min.saturating_sub(padding) as f64 / 1000.0 * h as f64) as u32;
                let x2 = ((loc.bbox_x_max + padding).min(1000) as f64 / 1000.0 * w as f64) as u32;
                let y2 = ((loc.bbox_y_max + padding).min(1000) as f64 / 1000.0 * h as f64) as u32;
                let cw = x2.saturating_sub(x1).max(1);
                let ch = y2.saturating_sub(y1).max(1);
                let cropped = img.crop_imm(x1, y1, cw, ch);

                let safe_label = sanitize_label(&loc.label);
                let filename = format!("{}_{}_p{}_{}.png", pdf_stem, i + 1, loc.page, safe_label);
                let out_path = out_dir.join(&filename);
                cropped
                    .save(&out_path)
                    .with_context(|| format!("saving {}", out_path.display()))?;
                eprintln!("[{}] Saved: {}", log_prefix, out_path.display());
                outputs.push((loc.label.clone(), out_path));
                saved += 1;
            }
        }
    }

    Ok((saved, outputs))
}

// ── Helpers ───────────────────────────────────────────────────────────

pub fn sanitize_label(label: &str) -> String {
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

pub fn make_attachment(
    pdf: &Path,
    no_cache: bool,
    api_key: &str,
    base_url: &Option<String>,
) -> Result<AttachmentSource> {
    if no_cache {
        let data = std::fs::read(pdf).with_context(|| format!("reading {}", pdf.display()))?;
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
