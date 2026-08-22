//! SVD (System View Description) file search and download from cmsis-svd/cmsis-svd-data.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const GITHUB_TREES_URL: &str =
    "https://api.github.com/repos/cmsis-svd/cmsis-svd-data/git/trees/main?recursive=1";
const RAW_BASE_URL: &str =
    "https://raw.githubusercontent.com/cmsis-svd/cmsis-svd-data/main";
const USER_AGENT: &str = "datasheet-cli/0.1 (https://github.com/akiselev/datasheet-cli)";

/// SVD subcommands.
#[derive(Subcommand, Debug)]
pub enum SvdSubcommand {
    /// Search for SVD files by chip name
    Search {
        /// Chip name or partial match (e.g., "STM32F407", "rp2350", "esp32c6")
        query: String,
        /// Filter by vendor
        #[arg(long)]
        vendor: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Download an SVD file for a chip
    Download {
        /// Chip name (e.g., "STM32F407", "rp2350", "esp32c6")
        chip: String,
        /// Filter by vendor (useful when chip name is ambiguous)
        #[arg(long)]
        vendor: Option<String>,
        /// Output file path (defaults to <chip>.svd in current directory)
        #[arg(long, short)]
        out: Option<PathBuf>,
    },
    /// List all available vendors
    Vendors {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct SvdEntry {
    vendor: String,
    filename: String,
    chip: String,
    path: String,
}

#[derive(Deserialize)]
struct GitTreeResponse {
    tree: Vec<GitTreeEntry>,
}

#[derive(Deserialize)]
struct GitTreeEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
}

// --- Cache ---

fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("datasheet-cli").join("svd"))
}

fn get_cached_index() -> Option<Vec<SvdEntry>> {
    let path = cache_dir()?.join("index.json");
    let metadata = std::fs::metadata(&path).ok()?;
    let age = metadata.modified().ok()?.elapsed().ok()?;
    if age > std::time::Duration::from_secs(86400) {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn set_cached_index(entries: &[SvdEntry]) {
    let Some(dir) = cache_dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("index.json");
    if let Ok(json) = serde_json::to_string(entries) {
        let _ = std::fs::write(path, json);
    }
}

// --- Index fetching ---

fn fetch_svd_index() -> Result<Vec<SvdEntry>, String> {
    if let Some(cached) = get_cached_index() {
        return Ok(cached);
    }

    eprintln!("Fetching SVD index from GitHub (this may take a moment)...");

    let response = ureq::get(GITHUB_TREES_URL)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(403, _) => {
                "GitHub API rate limit exceeded. Wait a few minutes and try again.".to_string()
            }
            other => format!("Failed to fetch SVD index: {other}"),
        })?;

    let tree_response: GitTreeResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse GitHub API response: {e}"))?;

    let entries: Vec<SvdEntry> = tree_response
        .tree
        .into_iter()
        .filter(|e| {
            e.entry_type == "blob"
                && e.path.starts_with("data/")
                && e.path.ends_with(".svd")
        })
        .filter_map(|e| {
            // path is like "data/<Vendor>/<chip>.svd"
            let parts: Vec<&str> = e.path.splitn(3, '/').collect();
            if parts.len() != 3 {
                return None;
            }
            let vendor = parts[1].to_string();
            let filename = parts[2].to_string();
            let chip = filename.trim_end_matches(".svd").to_string();
            Some(SvdEntry {
                vendor,
                filename,
                chip,
                path: e.path,
            })
        })
        .collect();

    set_cached_index(&entries);
    Ok(entries)
}

// --- Command handlers ---

fn cmd_search(query: &str, vendor: Option<&str>, json: bool) -> Result<(), String> {
    let index = fetch_svd_index()?;
    let query_lower = query.to_lowercase();

    let results: Vec<&SvdEntry> = index
        .iter()
        .filter(|e| e.chip.to_lowercase().contains(&query_lower))
        .filter(|e| {
            vendor
                .map(|v| e.vendor.to_lowercase().contains(&v.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
        return Ok(());
    }

    if results.is_empty() {
        println!("No SVD files found matching \"{}\".", query);
        maybe_print_ch32_hint(query);
        return Ok(());
    }

    println!("Found {} SVD file{} matching \"{}\":\n", results.len(), if results.len() == 1 { "" } else { "s" }, query);
    for entry in &results {
        println!("  {}/{}", entry.vendor, entry.filename);
    }

    Ok(())
}

fn cmd_download(chip: &str, vendor: Option<&str>, out: Option<PathBuf>) -> Result<(), String> {
    let index = fetch_svd_index()?;
    let chip_lower = chip.to_lowercase();

    // Exact match first (case-insensitive), then substring
    let mut matches: Vec<&SvdEntry> = index
        .iter()
        .filter(|e| e.chip.to_lowercase() == chip_lower)
        .filter(|e| {
            vendor
                .map(|v| e.vendor.to_lowercase().contains(&v.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();

    if matches.is_empty() {
        matches = index
            .iter()
            .filter(|e| e.chip.to_lowercase().contains(&chip_lower))
            .filter(|e| {
                vendor
                    .map(|v| e.vendor.to_lowercase().contains(&v.to_lowercase()))
                    .unwrap_or(true)
            })
            .collect();
    }

    if matches.is_empty() {
        eprintln!("No SVD found for \"{}\".", chip);
        maybe_print_ch32_hint(chip);
        return Err(format!("No SVD found for \"{}\"", chip));
    }

    if matches.len() > 1 && vendor.is_none() {
        eprintln!(
            "Multiple SVD files match \"{}\". Use --vendor to disambiguate:\n",
            chip
        );
        for entry in &matches {
            eprintln!("  {}/{}", entry.vendor, entry.filename);
        }
        eprintln!("\nExample: datasheet svd download {} --vendor {}", chip, matches[0].vendor);
        return Err(format!("Ambiguous chip name \"{}\"", chip));
    }

    let entry = matches[0];
    let url = format!("{}/{}", RAW_BASE_URL, entry.path);
    let dest = out.unwrap_or_else(|| PathBuf::from(format!("{}.svd", entry.chip)));

    eprintln!("Downloading {} from cmsis-svd-data...", entry.filename);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let mut content = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut content)
        .map_err(|e| format!("Failed to read response: {e}"))?;

    std::fs::write(&dest, &content).map_err(|e| format!("Failed to write file: {e}"))?;

    println!("Downloaded {} ({} bytes) to {}", entry.filename, content.len(), dest.display());
    Ok(())
}

fn cmd_vendors(json: bool) -> Result<(), String> {
    let index = fetch_svd_index()?;

    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for entry in &index {
        *counts.entry(entry.vendor.clone()).or_insert(0) += 1;
    }

    if json {
        let vendors: Vec<serde_json::Value> = counts
            .iter()
            .map(|(v, c)| serde_json::json!({"vendor": v, "count": c}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&vendors).unwrap_or_default());
        return Ok(());
    }

    println!("Available SVD vendors ({}):\n", counts.len());
    for (vendor, count) in &counts {
        println!("  {:<24} {:>4} files", vendor, count);
    }

    Ok(())
}

fn maybe_print_ch32_hint(query: &str) {
    if query.to_lowercase().starts_with("ch32") {
        println!(
            "\nCH32V SVDs are available from the ch32-rs community project:\n  https://github.com/ch32-rs/ch32-rs/tree/main/svd"
        );
    }
}

// io::Read for into_reader
use std::io::Read as _;

pub fn execute(subcommand: SvdSubcommand) -> Result<(), String> {
    match subcommand {
        SvdSubcommand::Search { query, vendor, json } => {
            cmd_search(&query, vendor.as_deref(), json)
        }
        SvdSubcommand::Download { chip, vendor, out } => {
            cmd_download(&chip, vendor.as_deref(), out)
        }
        SvdSubcommand::Vendors { json } => cmd_vendors(json),
    }
}
