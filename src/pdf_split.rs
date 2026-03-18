// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

//! PDF splitting for large datasheets that exceed the Gemini page limit.
//!
//! When a PDF exceeds MAX_PAGES, it is split along TOC chapter boundaries
//! into multiple files. Split files are cached by content hash so subsequent
//! runs reuse the cached splits.

use anyhow::{Context, Result};
use lopdf::Document;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_PAGES: u32 = 1000;

/// Information about how a PDF was split
#[derive(Debug)]
pub struct SplitResult {
    pub parts: Vec<SplitPart>,
}

#[derive(Debug)]
pub struct SplitPart {
    pub path: PathBuf,
    pub start_page: u32,
    pub end_page: u32,
}

/// Check if a PDF needs splitting and return the split parts if so.
/// Returns None if the PDF is within the page limit.
pub fn split_if_needed(pdf_path: &Path) -> Result<Option<SplitResult>> {
    // Load the document once for page count + TOC
    let doc = Document::load(pdf_path)
        .with_context(|| format!("loading PDF: {}", pdf_path.display()))?;

    let pages = doc.get_pages();
    let page_count = pages.len() as u32;

    if page_count <= MAX_PAGES {
        return Ok(None);
    }

    eprintln!(
        "[SPLIT] PDF has {} pages (limit: {}), splitting...",
        page_count, MAX_PAGES
    );

    // Check cache first (hash the file, not the parsed doc)
    let file_data = std::fs::read(pdf_path)
        .with_context(|| format!("reading {}", pdf_path.display()))?;
    let hash = compute_hash(&file_data);
    let cache_dir = get_split_cache_dir()?.join(&hash[..16]);

    if let Some(cached) = check_cached_splits(&cache_dir, page_count)? {
        eprintln!("[SPLIT] Using cached split ({} parts)", cached.parts.len());
        return Ok(Some(cached));
    }

    // Parse TOC for intelligent splitting
    let toc = read_toc_from_doc(&doc);
    let ranges = compute_split_ranges(page_count, &toc);

    eprintln!(
        "[SPLIT] Splitting into {} parts: {}",
        ranges.len(),
        ranges
            .iter()
            .map(|(s, e)| format!("p.{}-{}", s, e))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Create cache directory
    std::fs::create_dir_all(&cache_dir).context("creating split cache directory")?;

    // Split the PDF using fast page-keep approach
    let mut parts = Vec::new();
    for (i, (start, end)) in ranges.iter().enumerate() {
        let part_path = cache_dir.join(format!("part-{}.pdf", i + 1));

        split_pdf_fast(&file_data, &pages, *start, *end, &part_path)?;

        eprintln!(
            "[SPLIT] Saved pages {}-{} ({} pages) -> {}",
            start,
            end,
            end - start + 1,
            part_path.file_name().unwrap_or_default().to_string_lossy()
        );

        parts.push(SplitPart {
            path: part_path,
            start_page: *start,
            end_page: *end,
        });
    }

    save_split_metadata(&cache_dir, page_count, &ranges)?;

    Ok(Some(SplitResult { parts }))
}

/// Read the Table of Contents from an already-loaded document.
fn read_toc_from_doc(doc: &Document) -> Vec<(usize, u32)> {
    match doc.get_toc() {
        Ok(toc) => toc
            .toc
            .into_iter()
            .filter_map(|entry| {
                if entry.page > 0 {
                    Some((entry.level, entry.page as u32))
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Compute optimal split ranges that respect TOC chapter boundaries.
fn compute_split_ranges(total_pages: u32, toc: &[(usize, u32)]) -> Vec<(u32, u32)> {
    let num_parts = ((total_pages as f64) / (MAX_PAGES as f64)).ceil() as u32;

    if num_parts <= 1 {
        return vec![(1, total_pages)];
    }

    // Get top-level TOC entries as candidate split points
    let split_candidates: Vec<u32> = toc
        .iter()
        .filter(|(level, page)| *level <= 1 && *page > 1)
        .map(|(_, page)| *page)
        .collect();

    let mut split_points = Vec::new();

    for i in 1..num_parts {
        let ideal_page = (total_pages as f64 * i as f64 / num_parts as f64) as u32;

        if split_candidates.is_empty() {
            split_points.push(ideal_page);
        } else {
            let best = split_candidates
                .iter()
                .copied()
                .min_by_key(|&candidate| {
                    let distance = (candidate as i64 - ideal_page as i64).unsigned_abs();
                    if candidate > ideal_page + MAX_PAGES / 4 {
                        distance + 10000
                    } else {
                        distance
                    }
                })
                .unwrap_or(ideal_page);

            split_points.push(best);
        }
    }

    split_points.sort();
    split_points.dedup();

    // Build ranges, ensuring no part exceeds MAX_PAGES
    let mut ranges = Vec::new();
    let mut current_start = 1u32;

    for split_at in &split_points {
        if *split_at > current_start {
            let end = *split_at - 1;
            // If this range exceeds MAX_PAGES, force sub-splits
            let mut s = current_start;
            while end - s + 1 > MAX_PAGES {
                ranges.push((s, s + MAX_PAGES - 1));
                s += MAX_PAGES;
            }
            if s <= end {
                ranges.push((s, end));
            }
            current_start = end + 1;
        }
    }

    // Last range
    if current_start <= total_pages {
        let mut s = current_start;
        while total_pages - s + 1 > MAX_PAGES {
            ranges.push((s, s + MAX_PAGES - 1));
            s += MAX_PAGES;
        }
        ranges.push((s, total_pages));
    }

    ranges
}

/// Fast PDF split: load once, delete only the unwanted pages, skip expensive
/// object pruning and renumbering. This is much faster than the full
/// prune+renumber approach for large documents.
fn split_pdf_fast(
    pdf_data: &[u8],
    _all_pages: &BTreeMap<u32, lopdf::ObjectId>,
    start: u32,
    end: u32,
    output: &Path,
) -> Result<()> {
    let mut doc = Document::load_mem(pdf_data).context("loading PDF for splitting")?;

    let total_pages = doc.get_pages().len() as u32;

    // Delete pages outside the desired range.
    // Delete in reverse order to avoid invalidating page numbers.
    let pages_to_delete: Vec<u32> = (1..=total_pages)
        .filter(|&p| p < start || p > end)
        .rev()
        .collect();

    for &page_num in &pages_to_delete {
        doc.delete_pages(&[page_num]);
    }

    // Skip prune_objects and renumber_objects — they are the slow part.
    // The resulting PDF will have orphaned objects (larger file) but is
    // still valid and readable. Since these are temporary cached files
    // only used for Gemini upload, size doesn't matter much.

    doc.save(output)
        .with_context(|| format!("saving split PDF: {}", output.display()))?;

    Ok(())
}

fn check_cached_splits(cache_dir: &Path, expected_total: u32) -> Result<Option<SplitResult>> {
    let meta_path = cache_dir.join("split_meta.json");
    if !meta_path.exists() {
        return Ok(None);
    }

    let meta_str = std::fs::read_to_string(&meta_path).context("reading split metadata")?;
    let meta: serde_json::Value =
        serde_json::from_str(&meta_str).context("parsing split metadata")?;

    let stored_total = meta
        .get("total_pages")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if stored_total != expected_total {
        return Ok(None);
    }

    let ranges = meta
        .get("ranges")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    let start = r.get(0)?.as_u64()? as u32;
                    let end = r.get(1)?.as_u64()? as u32;
                    Some((start, end))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if ranges.is_empty() {
        return Ok(None);
    }

    let mut parts = Vec::new();
    for (i, (start, end)) in ranges.iter().enumerate() {
        let part_path = cache_dir.join(format!("part-{}.pdf", i + 1));
        if !part_path.exists() {
            return Ok(None);
        }
        parts.push(SplitPart {
            path: part_path,
            start_page: *start,
            end_page: *end,
        });
    }

    Ok(Some(SplitResult { parts }))
}

fn save_split_metadata(cache_dir: &Path, total_pages: u32, ranges: &[(u32, u32)]) -> Result<()> {
    let meta = serde_json::json!({
        "total_pages": total_pages,
        "ranges": ranges.iter().map(|(s, e)| vec![*s, *e]).collect::<Vec<_>>(),
    });

    let meta_path = cache_dir.join("split_meta.json");
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)
        .context("writing split metadata")?;

    Ok(())
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

fn get_split_cache_dir() -> Result<PathBuf> {
    if let Some(cache_dir) = dirs::cache_dir() {
        return Ok(cache_dir.join("datasheet-cli").join("splits"));
    }
    Ok(PathBuf::from(".cache").join("datasheet-cli").join("splits"))
}
