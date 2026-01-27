//! Mouser Electronics API integration commands.
//!
//! Provides CLI commands for searching electronic components and downloading datasheets
//! via the Mouser API.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

const MOUSER_API_BASE: &str = "https://api.mouser.com/api/v1";
const ENV_VAR_NAME: &str = "MOUSER_API_KEY";

/// Mouser API subcommands.
#[derive(Subcommand, Debug)]
pub enum MouserSubcommand {
    /// Search for parts by keyword
    Search {
        /// Search query (part number, keyword, or description)
        query: String,

        /// Mouser API key (defaults to MOUSER_API_KEY env var)
        #[arg(long, env = "MOUSER_API_KEY")]
        api_key: Option<String>,

        /// Maximum number of results to return (max 50)
        #[arg(long, short, default_value = "10")]
        limit: usize,

        /// Page number (1-indexed, takes precedence over --offset)
        #[arg(long, short)]
        page: Option<usize>,

        /// Starting record offset (0-indexed)
        #[arg(long, short = 'o')]
        offset: Option<usize>,

        /// Search by exact part number instead of keyword
        #[arg(long, short)]
        exact: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Download datasheet for a part
    Download {
        /// Part number to download datasheet for
        part_number: String,

        /// Mouser API key (defaults to MOUSER_API_KEY env var)
        #[arg(long, env = "MOUSER_API_KEY")]
        api_key: Option<String>,

        /// Output file path (defaults to <part_number>.pdf)
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Output directory (used if --output not specified)
        #[arg(long, short)]
        dir: Option<PathBuf>,
    },

    /// Get detailed information about a specific part
    Part {
        /// Mouser part number
        part_number: String,

        /// Mouser API key (defaults to MOUSER_API_KEY env var)
        #[arg(long, env = "MOUSER_API_KEY")]
        api_key: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

// Mouser API request/response types

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct KeywordSearchRequest {
    search_by_keyword_request: KeywordSearchBody,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct KeywordSearchBody {
    keyword: String,
    records: usize,
    starting_record: usize,
    search_options: Option<String>,
    search_with_y_our_sign_up_language: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PartNumberSearchRequest {
    search_by_part_request: PartNumberSearchBody,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PartNumberSearchBody {
    mouser_part_number: String,
    part_search_options: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SearchResponse {
    errors: Option<Vec<ApiError>>,
    search_results: Option<SearchResults>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct ApiError {
    id: Option<i32>,
    code: Option<String>,
    message: Option<String>,
    resource_key: Option<String>,
    resource_format_string: Option<String>,
    resource_message: Option<String>,
    property_name: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct SearchResults {
    number_of_result: Option<i32>,
    parts: Option<Vec<Part>>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Part {
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "LeadTime")]
    lead_time: Option<String>,
    #[serde(rename = "LifecycleStatus")]
    lifecycle_status: Option<String>,
    #[serde(rename = "Manufacturer")]
    manufacturer: Option<String>,
    #[serde(rename = "ManufacturerPartNumber")]
    manufacturer_part_number: Option<String>,
    #[serde(rename = "Min")]
    min: Option<String>,
    #[serde(rename = "Mult")]
    mult: Option<String>,
    #[serde(rename = "MouserPartNumber")]
    mouser_part_number: Option<String>,
    #[serde(rename = "ProductDetailUrl")]
    product_detail_url: Option<String>,
    #[serde(rename = "Reeling")]
    reeling: Option<bool>,
    #[serde(rename = "ROHSStatus")]
    rohs_status: Option<String>,
    #[serde(rename = "SuggestedReplacement")]
    suggested_replacement: Option<String>,
    #[serde(rename = "MultiSimBlue")]
    multi_sim_blue: Option<i32>,
    #[serde(rename = "AvailabilityInStock")]
    availability_in_stock: Option<String>,
    // AvailabilityOnOrder can be a string OR an array of objects in the API
    #[serde(rename = "AvailabilityOnOrder")]
    availability_on_order: Option<serde_json::Value>,
    #[serde(rename = "DataSheetUrl")]
    data_sheet_url: Option<String>,
    #[serde(rename = "PriceBreaks")]
    price_breaks: Option<Vec<PriceBreak>>,
    #[serde(rename = "AlternatePackagings")]
    alternate_packagings: Option<serde_json::Value>,
    // Additional fields that may be present in the API response
    #[serde(rename = "Availability")]
    availability: Option<String>,
    #[serde(rename = "Category")]
    category: Option<String>,
    #[serde(rename = "ImagePath")]
    image_path: Option<String>,
    #[serde(rename = "ProductAttributes")]
    product_attributes: Option<serde_json::Value>,
    #[serde(rename = "InfoMessages")]
    info_messages: Option<Vec<String>>,
    #[serde(rename = "REACH-SVHC")]
    reach_svhc: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PriceBreak {
    quantity: Option<i32>,
    price: Option<String>,
    currency: Option<String>,
}

/// Execute a Mouser subcommand.
pub fn execute(command: MouserSubcommand) -> Result<(), String> {
    match command {
        MouserSubcommand::Search {
            query,
            api_key,
            limit,
            page,
            offset,
            exact,
            json,
        } => cmd_search(&query, api_key.as_deref(), limit, page, offset, exact, json),
        MouserSubcommand::Download {
            part_number,
            api_key,
            output,
            dir,
        } => cmd_download(&part_number, api_key.as_deref(), output, dir),
        MouserSubcommand::Part {
            part_number,
            api_key,
            json,
        } => cmd_part(&part_number, api_key.as_deref(), json),
    }
}

fn get_api_key(provided: Option<&str>) -> Result<String, String> {
    if let Some(key) = provided {
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    std::env::var(ENV_VAR_NAME).map_err(|_| {
        format!(
            "Mouser API key not provided. Set {} environment variable or use --api-key",
            ENV_VAR_NAME
        )
    })
}

fn cmd_search(
    query: &str,
    api_key: Option<&str>,
    limit: usize,
    page: Option<usize>,
    offset: Option<usize>,
    exact: bool,
    json_output: bool,
) -> Result<(), String> {
    let api_key = get_api_key(api_key)?;

    // Calculate starting record: page takes precedence over offset
    let starting_record = if let Some(p) = page {
        if p == 0 {
            return Err("Page number must be 1 or greater".to_string());
        }
        (p - 1) * limit
    } else {
        offset.unwrap_or(0)
    };

    let parts = if exact {
        search_by_part_number(&api_key, query)?
    } else {
        search_by_keyword(&api_key, query, limit, starting_record)?
    };

    if json_output {
        let json = serde_json::to_string_pretty(&parts)
            .map_err(|e| format!("Failed to serialize results: {}", e))?;
        println!("{}", json);
    } else {
        if parts.is_empty() {
            println!("No parts found for query: {}", query);
            return Ok(());
        }

        println!("Found {} part(s):\n", parts.len());

        for (i, part) in parts.iter().take(limit).enumerate() {
            println!("{}. {}", i + 1, format_part_summary(part));
            println!();
        }
    }

    Ok(())
}

fn cmd_download(
    part_number: &str,
    api_key: Option<&str>,
    output: Option<PathBuf>,
    dir: Option<PathBuf>,
) -> Result<(), String> {
    let api_key = get_api_key(api_key)?;

    // Search for the part to get the datasheet URL
    let parts = search_by_part_number(&api_key, part_number)?;

    if parts.is_empty() {
        return Err(format!("Part not found: {}", part_number));
    }

    let part = &parts[0];
    let datasheet_url = part
        .data_sheet_url
        .as_ref()
        .ok_or_else(|| format!("No datasheet available for part: {}", part_number))?;

    if datasheet_url.is_empty() {
        return Err(format!("No datasheet available for part: {}", part_number));
    }

    // Determine output path
    let output_path = if let Some(path) = output {
        path
    } else {
        let filename = format!(
            "{}.pdf",
            part.manufacturer_part_number
                .as_ref()
                .unwrap_or(&part_number.to_string())
                .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        );
        if let Some(dir) = dir {
            dir.join(filename)
        } else {
            PathBuf::from(filename)
        }
    };

    println!("Downloading datasheet for {}...", part_number);
    println!("  URL: {}", datasheet_url);
    println!("  Output: {}", output_path.display());

    // Download the datasheet
    let response = ureq::get(datasheet_url)
        .call()
        .map_err(|e| format!("Failed to download datasheet: {}", e))?;

    let mut file =
        File::create(&output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file)
        .map_err(|e| format!("Failed to write datasheet: {}", e))?;

    println!("Datasheet downloaded successfully!");

    Ok(())
}

fn cmd_part(part_number: &str, api_key: Option<&str>, json_output: bool) -> Result<(), String> {
    let api_key = get_api_key(api_key)?;

    let parts = search_by_part_number(&api_key, part_number)?;

    if parts.is_empty() {
        return Err(format!("Part not found: {}", part_number));
    }

    let part = &parts[0];

    if json_output {
        let json = serde_json::to_string_pretty(part)
            .map_err(|e| format!("Failed to serialize part: {}", e))?;
        println!("{}", json);
    } else {
        print_part_details(part);
    }

    Ok(())
}

fn search_by_keyword(api_key: &str, keyword: &str, limit: usize, starting_record: usize) -> Result<Vec<Part>, String> {
    let url = format!("{}/search/keyword?apiKey={}", MOUSER_API_BASE, api_key);

    let request = KeywordSearchRequest {
        search_by_keyword_request: KeywordSearchBody {
            keyword: keyword.to_string(),
            records: limit,
            starting_record,
            search_options: None,
            search_with_y_our_sign_up_language: None,
        },
    };

    let response: SearchResponse = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(&request)
        .map_err(|e| format!("API request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    if let Some(errors) = response.errors {
        if !errors.is_empty() {
            let error_msgs: Vec<String> = errors
                .iter()
                .filter_map(|e| e.message.clone())
                .collect();
            if !error_msgs.is_empty() {
                return Err(format!("API errors: {}", error_msgs.join(", ")));
            }
        }
    }

    Ok(response
        .search_results
        .and_then(|r| r.parts)
        .unwrap_or_default())
}

fn search_by_part_number(api_key: &str, part_number: &str) -> Result<Vec<Part>, String> {
    let url = format!("{}/search/partnumber?apiKey={}", MOUSER_API_BASE, api_key);

    let request = PartNumberSearchRequest {
        search_by_part_request: PartNumberSearchBody {
            mouser_part_number: part_number.to_string(),
            part_search_options: None,
        },
    };

    let response: SearchResponse = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(&request)
        .map_err(|e| format!("API request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    if let Some(errors) = response.errors {
        if !errors.is_empty() {
            let error_msgs: Vec<String> = errors
                .iter()
                .filter_map(|e| e.message.clone())
                .collect();
            if !error_msgs.is_empty() {
                return Err(format!("API errors: {}", error_msgs.join(", ")));
            }
        }
    }

    Ok(response
        .search_results
        .and_then(|r| r.parts)
        .unwrap_or_default())
}

fn format_part_summary(part: &Part) -> String {
    let mut lines = Vec::new();

    if let Some(ref mpn) = part.manufacturer_part_number {
        if let Some(ref mfr) = part.manufacturer {
            lines.push(format!("{} ({})", mpn, mfr));
        } else {
            lines.push(mpn.clone());
        }
    } else if let Some(ref mouser_pn) = part.mouser_part_number {
        lines.push(format!("Mouser: {}", mouser_pn));
    }

    if let Some(ref desc) = part.description {
        lines.push(format!("   {}", desc));
    }

    if let Some(ref stock) = part.availability_in_stock {
        lines.push(format!("   Stock: {}", stock));
    }

    if let Some(ref prices) = part.price_breaks {
        if let Some(first) = prices.first() {
            if let (Some(qty), Some(price)) = (&first.quantity, &first.price) {
                let currency = first.currency.as_deref().unwrap_or("USD");
                lines.push(format!("   Price: {} {} (qty {}+)", price, currency, qty));
            }
        }
    }

    if part.data_sheet_url.as_ref().is_some_and(|u| !u.is_empty()) {
        lines.push("   Datasheet: Available".to_string());
    }

    lines.join("\n")
}

fn print_part_details(part: &Part) {
    println!("Part Details");
    println!("============");

    if let Some(ref mpn) = part.manufacturer_part_number {
        println!("Manufacturer Part Number: {}", mpn);
    }
    if let Some(ref mfr) = part.manufacturer {
        println!("Manufacturer: {}", mfr);
    }
    if let Some(ref mouser_pn) = part.mouser_part_number {
        println!("Mouser Part Number: {}", mouser_pn);
    }
    if let Some(ref desc) = part.description {
        println!("Description: {}", desc);
    }
    if let Some(ref status) = part.lifecycle_status {
        println!("Lifecycle Status: {}", status);
    }
    if let Some(ref rohs) = part.rohs_status {
        println!("RoHS Status: {}", rohs);
    }

    println!();
    println!("Availability");
    println!("------------");
    if let Some(ref stock) = part.availability_in_stock {
        println!("In Stock: {}", stock);
    }
    if let Some(ref on_order) = part.availability_on_order {
        if !on_order.is_null() {
            // Format value appropriately - could be string or array
            let display = match on_order {
                serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
                serde_json::Value::Array(arr) if !arr.is_empty() => {
                    Some(serde_json::to_string(arr).unwrap_or_default())
                }
                _ => None,
            };
            if let Some(val) = display {
                println!("On Order: {}", val);
            }
        }
    }
    if let Some(ref lead_time) = part.lead_time {
        println!("Lead Time: {}", lead_time);
    }
    if let Some(ref min) = part.min {
        println!("Minimum Order: {}", min);
    }
    if let Some(ref mult) = part.mult {
        println!("Order Multiple: {}", mult);
    }

    if let Some(ref prices) = part.price_breaks {
        if !prices.is_empty() {
            println!();
            println!("Pricing");
            println!("-------");
            for pb in prices {
                if let (Some(qty), Some(price)) = (&pb.quantity, &pb.price) {
                    let currency = pb.currency.as_deref().unwrap_or("USD");
                    println!("  {:>6}+ : {} {}", qty, price, currency);
                }
            }
        }
    }

    println!();
    println!("Links");
    println!("-----");
    if let Some(ref url) = part.product_detail_url {
        println!("Product Page: {}", url);
    }
    if let Some(ref url) = part.data_sheet_url {
        if !url.is_empty() {
            println!("Datasheet: {}", url);
        } else {
            println!("Datasheet: Not available");
        }
    } else {
        println!("Datasheet: Not available");
    }
}
