//! DigiKey Electronics API integration commands.
//!
//! Provides CLI commands for searching electronic components and downloading datasheets
//! via the DigiKey API v4.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

const DIGIKEY_API_BASE: &str = "https://api.digikey.com";
const DIGIKEY_API_BASE_SANDBOX: &str = "https://sandbox-api.digikey.com";
const ENV_VAR_CLIENT_ID: &str = "DIGIKEY_CLIENT_ID";
const ENV_VAR_CLIENT_SECRET: &str = "DIGIKEY_CLIENT_SECRET";

/// DigiKey API subcommands.
#[derive(Subcommand, Debug)]
pub enum DigikeySubcommand {
    /// Search for parts by keyword
    Search {
        /// Search query (part number, keyword, or description)
        query: String,

        /// DigiKey Client ID (defaults to DIGIKEY_CLIENT_ID env var)
        #[arg(long, env = "DIGIKEY_CLIENT_ID")]
        client_id: Option<String>,

        /// DigiKey Client Secret (defaults to DIGIKEY_CLIENT_SECRET env var)
        #[arg(long, env = "DIGIKEY_CLIENT_SECRET")]
        client_secret: Option<String>,

        /// Maximum number of results to return
        #[arg(long, short, default_value = "10")]
        limit: usize,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Use sandbox API for testing
        #[arg(long)]
        sandbox: bool,
    },

    /// Download datasheet for a part
    Download {
        /// Part number to download datasheet for
        part_number: String,

        /// DigiKey Client ID (defaults to DIGIKEY_CLIENT_ID env var)
        #[arg(long, env = "DIGIKEY_CLIENT_ID")]
        client_id: Option<String>,

        /// DigiKey Client Secret (defaults to DIGIKEY_CLIENT_SECRET env var)
        #[arg(long, env = "DIGIKEY_CLIENT_SECRET")]
        client_secret: Option<String>,

        /// Output file path (defaults to <part_number>.pdf)
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Output directory (used if --output not specified)
        #[arg(long, short)]
        dir: Option<PathBuf>,

        /// Use sandbox API for testing
        #[arg(long)]
        sandbox: bool,
    },

    /// Get detailed information about a specific part
    Part {
        /// DigiKey part number or manufacturer part number
        part_number: String,

        /// DigiKey Client ID (defaults to DIGIKEY_CLIENT_ID env var)
        #[arg(long, env = "DIGIKEY_CLIENT_ID")]
        client_id: Option<String>,

        /// DigiKey Client Secret (defaults to DIGIKEY_CLIENT_SECRET env var)
        #[arg(long, env = "DIGIKEY_CLIENT_SECRET")]
        client_secret: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Use sandbox API for testing
        #[arg(long)]
        sandbox: bool,
    },
}

// DigiKey API OAuth token types

#[derive(Serialize)]
struct TokenRequest {
    client_id: String,
    client_secret: String,
    grant_type: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
}

// DigiKey API request/response types

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct KeywordSearchRequest {
    keywords: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    record_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    record_start_position: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SearchResponse {
    products: Vec<Product>,
    #[serde(default)]
    products_count: i32,
    #[serde(default)]
    exact_manufacturer_products_count: i32,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Product {
    digi_key_part_number: Option<String>,
    manufacturer_part_number: Option<String>,
    manufacturer: Option<Manufacturer>,
    product_description: Option<String>,
    detailed_description: Option<String>,
    data_sheet_url: Option<String>,
    product_url: Option<String>,
    primary_photo: Option<String>,
    quantity_available: Option<i32>,
    minimum_order_quantity: Option<i32>,
    standard_pricing: Option<Vec<PriceBreak>>,
    unit_price: Option<f64>,
    manufacturer_public_quantity: Option<i32>,
    packaging: Option<PackagingInfo>,
    ro_hs_status: Option<String>,
    lead_status: Option<String>,
    part_status: Option<String>,
    parameters: Option<Vec<Parameter>>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Manufacturer {
    name: Option<String>,
    id: Option<i32>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PriceBreak {
    break_quantity: Option<i32>,
    unit_price: Option<f64>,
    total_price: Option<f64>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PackagingInfo {
    value: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Parameter {
    parameter: Option<String>,
    value: Option<String>,
}

/// Execute a DigiKey subcommand.
pub fn execute(command: DigikeySubcommand) -> Result<(), String> {
    match command {
        DigikeySubcommand::Search {
            query,
            client_id,
            client_secret,
            limit,
            json,
            sandbox,
        } => cmd_search(&query, client_id.as_deref(), client_secret.as_deref(), limit, json, sandbox),
        DigikeySubcommand::Download {
            part_number,
            client_id,
            client_secret,
            output,
            dir,
            sandbox,
        } => cmd_download(&part_number, client_id.as_deref(), client_secret.as_deref(), output, dir, sandbox),
        DigikeySubcommand::Part {
            part_number,
            client_id,
            client_secret,
            json,
            sandbox,
        } => cmd_part(&part_number, client_id.as_deref(), client_secret.as_deref(), json, sandbox),
    }
}

fn get_credentials(
    provided_client_id: Option<&str>,
    provided_client_secret: Option<&str>,
) -> Result<(String, String), String> {
    let client_id = if let Some(id) = provided_client_id {
        if !id.is_empty() {
            id.to_string()
        } else {
            std::env::var(ENV_VAR_CLIENT_ID).map_err(|_| {
                format!(
                    "DigiKey Client ID not provided. Set {} environment variable or use --client-id",
                    ENV_VAR_CLIENT_ID
                )
            })?
        }
    } else {
        std::env::var(ENV_VAR_CLIENT_ID).map_err(|_| {
            format!(
                "DigiKey Client ID not provided. Set {} environment variable or use --client-id",
                ENV_VAR_CLIENT_ID
            )
        })?
    };

    let client_secret = if let Some(secret) = provided_client_secret {
        if !secret.is_empty() {
            secret.to_string()
        } else {
            std::env::var(ENV_VAR_CLIENT_SECRET).map_err(|_| {
                format!(
                    "DigiKey Client Secret not provided. Set {} environment variable or use --client-secret",
                    ENV_VAR_CLIENT_SECRET
                )
            })?
        }
    } else {
        std::env::var(ENV_VAR_CLIENT_SECRET).map_err(|_| {
            format!(
                "DigiKey Client Secret not provided. Set {} environment variable or use --client-secret",
                ENV_VAR_CLIENT_SECRET
            )
        })?
    };

    Ok((client_id, client_secret))
}

fn get_access_token(client_id: &str, client_secret: &str, sandbox: bool) -> Result<String, String> {
    let base_url = if sandbox { DIGIKEY_API_BASE_SANDBOX } else { DIGIKEY_API_BASE };
    let url = format!("{}/v1/oauth2/token", base_url);

    let response: TokenResponse = ureq::post(&url)
        .send_form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "client_credentials"),
        ])
        .map_err(|e| format!("Failed to get access token: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    Ok(response.access_token)
}

fn cmd_search(
    query: &str,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    limit: usize,
    json_output: bool,
    sandbox: bool,
) -> Result<(), String> {
    let (client_id, client_secret) = get_credentials(client_id, client_secret)?;
    let access_token = get_access_token(&client_id, &client_secret, sandbox)?;

    let products = search_by_keyword(&client_id, &access_token, query, limit, sandbox)?;

    if json_output {
        let json = serde_json::to_string_pretty(&products)
            .map_err(|e| format!("Failed to serialize results: {}", e))?;
        println!("{}", json);
    } else {
        if products.is_empty() {
            println!("No parts found for query: {}", query);
            return Ok(());
        }

        println!("Found {} part(s):\n", products.len());

        for (i, product) in products.iter().take(limit).enumerate() {
            println!("{}. {}", i + 1, format_product_summary(product));
            println!();
        }
    }

    Ok(())
}

fn cmd_download(
    part_number: &str,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    output: Option<PathBuf>,
    dir: Option<PathBuf>,
    sandbox: bool,
) -> Result<(), String> {
    let (client_id, client_secret) = get_credentials(client_id, client_secret)?;
    let access_token = get_access_token(&client_id, &client_secret, sandbox)?;

    // Get exact part details using the ProductDetails endpoint
    let product = get_part_by_number(&client_id, &access_token, part_number, sandbox)?;
    let datasheet_url = product
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
            product.manufacturer_part_number
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

    // Download the datasheet with proper headers (distributor CDNs require User-Agent)
    let response = ureq::get(datasheet_url)
        .set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .set("Accept", "application/pdf,*/*")
        .call()
        .map_err(|e| format!("Failed to download datasheet: {}", e))?;

    // Verify we got a PDF, not an HTML error/redirect page
    let content_type = response.content_type().to_string();

    let mut file =
        File::create(&output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut reader = response.into_reader();
    let bytes_written = std::io::copy(&mut reader, &mut file)
        .map_err(|e| format!("Failed to write datasheet: {}", e))?;

    // Check if we got HTML instead of PDF (bot protection / redirect)
    if content_type.contains("text/html") || bytes_written < 1024 {
        let _ = std::fs::remove_file(&output_path);
        return Err(format!(
            "Download returned HTML instead of PDF (content-type: {}). \
             Distributor may be blocking automated downloads for this URL.",
            content_type
        ));
    }

    println!("Datasheet downloaded successfully! ({:.1} KB)", bytes_written as f64 / 1024.0);

    Ok(())
}

fn cmd_part(
    part_number: &str,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    json_output: bool,
    sandbox: bool,
) -> Result<(), String> {
    let (client_id, client_secret) = get_credentials(client_id, client_secret)?;
    let access_token = get_access_token(&client_id, &client_secret, sandbox)?;

    // Get exact part details using the ProductDetails endpoint
    let product = get_part_by_number(&client_id, &access_token, part_number, sandbox)?;

    if json_output {
        let json = serde_json::to_string_pretty(&product)
            .map_err(|e| format!("Failed to serialize part: {}", e))?;
        println!("{}", json);
    } else {
        print_product_details(&product);
    }

    Ok(())
}

fn search_by_keyword(
    client_id: &str,
    access_token: &str,
    keyword: &str,
    limit: usize,
    sandbox: bool,
) -> Result<Vec<Product>, String> {
    let base_url = if sandbox { DIGIKEY_API_BASE_SANDBOX } else { DIGIKEY_API_BASE };
    let url = format!("{}/products/v4/search/keyword", base_url);

    let request = KeywordSearchRequest {
        keywords: keyword.to_string(),
        record_count: Some(limit),
        record_start_position: Some(0),
    };

    let response: SearchResponse = ureq::post(&url)
        .set("X-DIGIKEY-Client-Id", client_id)
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .send_json(&request)
        .map_err(|e| format!("API request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(response.products)
}

/// Get exact part details by part number using the ProductDetails endpoint.
/// This endpoint returns exact matches for DigiKey or manufacturer part numbers.
fn get_part_by_number(
    client_id: &str,
    access_token: &str,
    part_number: &str,
    sandbox: bool,
) -> Result<Product, String> {
    let base_url = if sandbox { DIGIKEY_API_BASE_SANDBOX } else { DIGIKEY_API_BASE };
    // URL encode the part number to handle special characters
    let encoded_part = urlencoding::encode(part_number);
    let url = format!("{}/products/v4/search/{}/productdetails", base_url, encoded_part);

    let product: Product = ureq::get(&url)
        .set("X-DIGIKEY-Client-Id", client_id)
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Accept", "application/json")
        .call()
        .map_err(|e| {
            match e {
                ureq::Error::Status(404, _) => {
                    format!("Part not found: {}", part_number)
                }
                _ => format!("API request failed: {}", e)
            }
        })?
        .into_json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(product)
}

fn format_product_summary(product: &Product) -> String {
    let mut lines = Vec::new();

    if let Some(ref mpn) = product.manufacturer_part_number {
        if let Some(ref mfr) = product.manufacturer {
            if let Some(ref name) = mfr.name {
                lines.push(format!("{} ({})", mpn, name));
            } else {
                lines.push(mpn.clone());
            }
        } else {
            lines.push(mpn.clone());
        }
    } else if let Some(ref dk_pn) = product.digi_key_part_number {
        lines.push(format!("DigiKey: {}", dk_pn));
    }

    if let Some(ref desc) = product.product_description {
        lines.push(format!("   {}", desc));
    }

    if let Some(qty) = product.quantity_available {
        lines.push(format!("   Stock: {}", qty));
    }

    if let Some(ref prices) = product.standard_pricing {
        if let Some(first) = prices.first() {
            if let (Some(qty), Some(price)) = (first.break_quantity, first.unit_price) {
                lines.push(format!("   Price: ${:.4} (qty {}+)", price, qty));
            }
        }
    }

    if product.data_sheet_url.as_ref().is_some_and(|u| !u.is_empty()) {
        lines.push("   Datasheet: Available".to_string());
    }

    lines.join("\n")
}

fn print_product_details(product: &Product) {
    println!("Part Details");
    println!("============");

    if let Some(ref mpn) = product.manufacturer_part_number {
        println!("Manufacturer Part Number: {}", mpn);
    }
    if let Some(ref mfr) = product.manufacturer {
        if let Some(ref name) = mfr.name {
            println!("Manufacturer: {}", name);
        }
    }
    if let Some(ref dk_pn) = product.digi_key_part_number {
        println!("DigiKey Part Number: {}", dk_pn);
    }
    if let Some(ref desc) = product.product_description {
        println!("Description: {}", desc);
    }
    if let Some(ref detailed) = product.detailed_description {
        println!("Detailed Description: {}", detailed);
    }
    if let Some(ref status) = product.part_status {
        println!("Part Status: {}", status);
    }
    if let Some(ref rohs) = product.ro_hs_status {
        println!("RoHS Status: {}", rohs);
    }
    if let Some(ref lead) = product.lead_status {
        println!("Lead Status: {}", lead);
    }

    println!();
    println!("Availability");
    println!("------------");
    if let Some(qty) = product.quantity_available {
        println!("In Stock: {}", qty);
    }
    if let Some(mfr_qty) = product.manufacturer_public_quantity {
        println!("Manufacturer Stock: {}", mfr_qty);
    }
    if let Some(min_qty) = product.minimum_order_quantity {
        println!("Minimum Order: {}", min_qty);
    }
    if let Some(ref packaging) = product.packaging {
        if let Some(ref value) = packaging.value {
            println!("Packaging: {}", value);
        }
    }

    if let Some(ref prices) = product.standard_pricing {
        if !prices.is_empty() {
            println!();
            println!("Pricing");
            println!("-------");
            for pb in prices {
                if let (Some(qty), Some(price)) = (pb.break_quantity, pb.unit_price) {
                    println!("  {:>6}+ : ${:.4}", qty, price);
                }
            }
        }
    }

    if let Some(ref params) = product.parameters {
        if !params.is_empty() {
            println!();
            println!("Parameters");
            println!("----------");
            for param in params.iter().take(10) {
                if let (Some(name), Some(value)) = (&param.parameter, &param.value) {
                    println!("  {}: {}", name, value);
                }
            }
            if params.len() > 10 {
                println!("  ... and {} more parameters", params.len() - 10);
            }
        }
    }

    println!();
    println!("Links");
    println!("-----");
    if let Some(ref url) = product.product_url {
        println!("Product Page: {}", url);
    }
    if let Some(ref url) = product.data_sheet_url {
        if !url.is_empty() {
            println!("Datasheet: {}", url);
        } else {
            println!("Datasheet: Not available");
        }
    } else {
        println!("Datasheet: Not available");
    }
}
