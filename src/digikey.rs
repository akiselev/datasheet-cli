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

        /// Filter by category name (resolved via discovery)
        #[arg(long, short)]
        category: Option<String>,

        /// Filter by parametric parameter, format "Name=Value" (repeatable)
        #[arg(long)]
        param: Vec<String>,

        /// Filter by manufacturer name (resolved via discovery)
        #[arg(long, short)]
        manufacturer: Option<String>,

        /// Only show in-stock parts
        #[arg(long)]
        in_stock: bool,

        /// Sort results: price, stock, mpn, manufacturer
        #[arg(long)]
        sort: Option<String>,

        /// Filter by category ID (direct, skips discovery)
        #[arg(long)]
        category_id: Option<i64>,

        /// Filter by manufacturer ID (direct, repeatable)
        #[arg(long)]
        manufacturer_id: Vec<i64>,

        /// Filter by parametric ID, format "ParameterId=ValueId" (repeatable)
        #[arg(long)]
        param_id: Vec<String>,

        /// Output FilterOptions from API response as JSON instead of products
        #[arg(long)]
        show_filters: bool,
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

    /// Quick stock and pricing check for a part
    Stock {
        /// Part number to check
        part_number: String,

        #[arg(long, env = "DIGIKEY_CLIENT_ID")]
        client_id: Option<String>,

        #[arg(long, env = "DIGIKEY_CLIENT_SECRET")]
        client_secret: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        #[arg(long)]
        sandbox: bool,
    },
}

// Normalized stock/pricing output type

#[derive(Serialize)]
struct StockInfo {
    mpn: String,
    manufacturer: Option<String>,
    distributor: &'static str,
    distributor_pn: Option<String>,
    lifecycle_status: Option<String>,
    stock: Option<i64>,
    lead_time: Option<String>,
    moq: Option<i32>,
    order_multiple: Option<i32>,
    currency: String,
    price_breaks: Vec<StockPriceBreak>,
}

#[derive(Serialize)]
struct StockPriceBreak {
    quantity: i32,
    unit_price: f64,
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
    limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_options_request: Option<FilterOptionsRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sort_options: Option<SortOptions>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct FilterOptionsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer_filter: Option<Vec<FilterId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category_filter: Option<Vec<FilterId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_quantity_available: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameter_filter_request: Option<ParameterFilterRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_options: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct ParameterFilterRequest {
    category_filter: FilterId,
    parameter_filters: Vec<ParametricFilter>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct ParametricFilter {
    parameter_id: i64,
    filter_values: Vec<FilterId>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct FilterId {
    id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SortOptions {
    field: String,
    sort_order: String,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct SearchResponse {
    products: Vec<Product>,
    #[serde(default)]
    products_count: i32,
    #[serde(default)]
    exact_manufacturer_products_count: i32,
    #[serde(default)]
    filter_options: Option<FilterOptions>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct FilterOptions {
    #[serde(default)]
    manufacturers: Vec<BaseFilter>,
    #[serde(default)]
    packaging: Vec<BaseFilter>,
    #[serde(default)]
    status: Vec<BaseFilter>,
    #[serde(default)]
    series: Vec<BaseFilter>,
    #[serde(default)]
    parametric_filters: Vec<ParametricFilterOption>,
    #[serde(default)]
    top_categories: Vec<TopCategory>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct BaseFilter {
    id: Option<i64>,
    value: Option<String>,
    product_count: Option<i64>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct ParametricFilterOption {
    parameter_id: Option<i64>,
    parameter_name: Option<String>,
    filter_values: Option<Vec<FilterValue>>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct FilterValue {
    value_id: Option<String>,
    value_name: Option<String>,
    product_count: Option<i64>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct TopCategory {
    root_category: Option<CategoryInfo>,
    category: Option<CategoryInfo>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct CategoryInfo {
    id: Option<i64>,
    name: Option<String>,
    product_count: Option<i64>,
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
            category,
            param,
            manufacturer,
            in_stock,
            sort,
            category_id,
            manufacturer_id,
            param_id,
            show_filters,
        } => cmd_search(
            &query,
            client_id.as_deref(),
            client_secret.as_deref(),
            limit,
            json,
            sandbox,
            category,
            param,
            manufacturer,
            in_stock,
            sort,
            category_id,
            manufacturer_id,
            param_id,
            show_filters,
        ),
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
        DigikeySubcommand::Stock {
            part_number,
            client_id,
            client_secret,
            json,
            sandbox,
        } => cmd_stock(&part_number, client_id.as_deref(), client_secret.as_deref(), json, sandbox),
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

fn map_sort_field(sort: &str) -> Result<SortOptions, String> {
    let (field, order) = match sort {
        "price" => ("Price", "Ascending"),
        "stock" => ("QuantityAvailable", "Descending"),
        "mpn" => ("ManufacturerProductNumber", "Ascending"),
        "manufacturer" => ("Manufacturer", "Ascending"),
        _ => {
            return Err(format!(
                "Unknown sort field: {}. Options: price, stock, mpn, manufacturer",
                sort
            ))
        }
    };
    Ok(SortOptions { field: field.to_string(), sort_order: order.to_string() })
}

fn build_search_options(in_stock: bool) -> Option<Vec<String>> {
    if in_stock {
        Some(vec!["InStock".to_string()])
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_search(
    query: &str,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    limit: usize,
    json_output: bool,
    sandbox: bool,
    category: Option<String>,
    params: Vec<String>,
    manufacturer: Option<String>,
    in_stock: bool,
    sort: Option<String>,
    category_id: Option<i64>,
    manufacturer_ids: Vec<i64>,
    param_ids: Vec<String>,
    show_filters: bool,
) -> Result<(), String> {
    let (client_id, client_secret) = get_credentials(client_id, client_secret)?;
    let access_token = get_access_token(&client_id, &client_secret, sandbox)?;

    let sort_options = sort.as_deref().map(map_sort_field).transpose()?;

    let has_name_filters = category.is_some() || !params.is_empty() || manufacturer.is_some();
    let has_id_filters =
        category_id.is_some() || !manufacturer_ids.is_empty() || !param_ids.is_empty();

    let response = if has_name_filters && !has_id_filters {
        // TWO-STEP: Discovery search to resolve names to IDs, then filtered search.

        let discovery =
            search_by_keyword(&client_id, &access_token, query, 1, sandbox, None, None)?;
        let filter_opts = discovery
            .filter_options
            .ok_or("API did not return filter options for discovery search")?;

        // Resolve category name → ID
        let resolved_category_id: Option<i64> = if let Some(ref cat_name) = category {
            let cat_lower = cat_name.to_lowercase();
            let matched = filter_opts.top_categories.iter().find(|tc| {
                tc.category
                    .as_ref()
                    .and_then(|c| c.name.as_ref())
                    .map(|n| {
                        n.eq_ignore_ascii_case(cat_name) || n.to_lowercase().contains(&cat_lower)
                    })
                    .unwrap_or(false)
            });
            match matched {
                Some(tc) => tc.category.as_ref().and_then(|c| c.id),
                None => {
                    let available: Vec<String> = filter_opts
                        .top_categories
                        .iter()
                        .filter_map(|tc| tc.category.as_ref().and_then(|c| c.name.clone()))
                        .collect();
                    return Err(format!(
                        "Category '{}' not found. Available: {}",
                        cat_name,
                        available.join(", ")
                    ));
                }
            }
        } else {
            None
        };

        // Resolve manufacturer name → ID
        let resolved_mfr_ids: Option<Vec<FilterId>> = if let Some(ref mfr_name) = manufacturer {
            let mfr_lower = mfr_name.to_lowercase();
            let matched = filter_opts.manufacturers.iter().find(|m| {
                m.value
                    .as_ref()
                    .map(|v| {
                        v.eq_ignore_ascii_case(mfr_name) || v.to_lowercase().contains(&mfr_lower)
                    })
                    .unwrap_or(false)
            });
            match matched {
                Some(m) => Some(vec![FilterId {
                    id: m.id.map(|i| i.to_string()).unwrap_or_default(),
                }]),
                None => {
                    let available: Vec<String> = filter_opts
                        .manufacturers
                        .iter()
                        .filter_map(|m| m.value.clone())
                        .take(20)
                        .collect();
                    return Err(format!(
                        "Manufacturer '{}' not found. Top matches: {}",
                        mfr_name,
                        available.join(", ")
                    ));
                }
            }
        } else {
            None
        };

        // Resolve --param "Name=Value" → ParameterId + ValueId
        let mut resolved_params: Vec<ParametricFilter> = Vec::new();
        for param_str in &params {
            let (name_part, value_part) = param_str
                .split_once('=')
                .ok_or_else(|| format!("Invalid --param format '{}': expected 'Name=Value'", param_str))?;
            let name_lower = name_part.trim().to_lowercase();
            let value_lower = value_part.trim().to_lowercase();

            let param_opt = filter_opts.parametric_filters.iter().find(|pf| {
                pf.parameter_name
                    .as_ref()
                    .map(|n| {
                        n.eq_ignore_ascii_case(name_part.trim())
                            || n.to_lowercase().contains(&name_lower)
                    })
                    .unwrap_or(false)
            });

            let param_opt = match param_opt {
                Some(p) => p,
                None => {
                    let available: Vec<String> = filter_opts
                        .parametric_filters
                        .iter()
                        .filter_map(|pf| pf.parameter_name.clone())
                        .collect();
                    return Err(format!(
                        "Parameter '{}' not found. Available: {}",
                        name_part.trim(),
                        available.join(", ")
                    ));
                }
            };

            let parameter_id = param_opt
                .parameter_id
                .ok_or_else(|| format!("Parameter '{}' has no ID", name_part.trim()))?;

            let filter_vals = param_opt.filter_values.as_deref().unwrap_or(&[]);
            let matched_val = filter_vals.iter().find(|fv| {
                fv.value_name
                    .as_ref()
                    .map(|n| {
                        n.eq_ignore_ascii_case(value_part.trim())
                            || n.to_lowercase().contains(&value_lower)
                    })
                    .unwrap_or(false)
            });

            let matched_val = match matched_val {
                Some(v) => v,
                None => {
                    let available: Vec<String> = filter_vals
                        .iter()
                        .filter_map(|fv| fv.value_name.clone())
                        .collect();
                    return Err(format!(
                        "Value '{}' not found for parameter '{}'. Available: {}",
                        value_part.trim(),
                        name_part.trim(),
                        available.join(", ")
                    ));
                }
            };

            let value_id = matched_val
                .value_id
                .clone()
                .ok_or_else(|| format!("Value '{}' has no ID", value_part.trim()))?;

            resolved_params.push(ParametricFilter {
                parameter_id,
                filter_values: vec![FilterId { id: value_id }],
            });
        }

        // Build FilterOptionsRequest
        let parameter_filter_request = if !resolved_params.is_empty() {
            let cat_id = resolved_category_id.ok_or(
                "--category is required when using --param (DigiKey requires a category for parametric filtering)",
            )?;
            Some(ParameterFilterRequest {
                category_filter: FilterId { id: cat_id.to_string() },
                parameter_filters: resolved_params,
            })
        } else {
            None
        };

        let filter_request = FilterOptionsRequest {
            manufacturer_filter: resolved_mfr_ids,
            category_filter: resolved_category_id
                .map(|id| vec![FilterId { id: id.to_string() }]),
            minimum_quantity_available: None,
            parameter_filter_request,
            search_options: build_search_options(in_stock),
        };

        search_by_keyword(
            &client_id,
            &access_token,
            query,
            limit,
            sandbox,
            Some(filter_request),
            sort_options,
        )?
    } else if has_id_filters {
        // DIRECT: Use provided IDs without discovery.

        let manufacturer_filter = if !manufacturer_ids.is_empty() {
            Some(manufacturer_ids.iter().map(|id| FilterId { id: id.to_string() }).collect())
        } else {
            None
        };

        let category_filter = category_id
            .map(|id| vec![FilterId { id: id.to_string() }]);

        // Parse --param-id "ParameterId=ValueId"
        let mut direct_params: Vec<ParametricFilter> = Vec::new();
        for pid_str in &param_ids {
            let (param_id_str, value_id_str) = pid_str.split_once('=').ok_or_else(|| {
                format!(
                    "Invalid --param-id format '{}': expected 'ParameterId=ValueId'",
                    pid_str
                )
            })?;
            let parameter_id: i64 = param_id_str.trim().parse().map_err(|_| {
                format!("Invalid parameter ID '{}': must be an integer", param_id_str.trim())
            })?;
            direct_params.push(ParametricFilter {
                parameter_id,
                filter_values: vec![FilterId { id: value_id_str.trim().to_string() }],
            });
        }

        let parameter_filter_request = if !direct_params.is_empty() {
            let cat_id = category_id.ok_or(
                "--category-id is required when using --param-id (DigiKey requires a category for parametric filtering)",
            )?;
            Some(ParameterFilterRequest {
                category_filter: FilterId { id: cat_id.to_string() },
                parameter_filters: direct_params,
            })
        } else {
            None
        };

        let filter_request = FilterOptionsRequest {
            manufacturer_filter,
            category_filter,
            minimum_quantity_available: None,
            parameter_filter_request,
            search_options: build_search_options(in_stock),
        };

        search_by_keyword(
            &client_id,
            &access_token,
            query,
            limit,
            sandbox,
            Some(filter_request),
            sort_options,
        )?
    } else {
        // SIMPLE: No filters, keyword search only.
        let filter_request = if in_stock {
            Some(FilterOptionsRequest {
                manufacturer_filter: None,
                category_filter: None,
                minimum_quantity_available: None,
                parameter_filter_request: None,
                search_options: build_search_options(in_stock),
            })
        } else {
            None
        };

        search_by_keyword(
            &client_id,
            &access_token,
            query,
            limit,
            sandbox,
            filter_request,
            sort_options,
        )?
    };

    if show_filters {
        let json = serde_json::to_string_pretty(&response.filter_options)
            .map_err(|e| format!("Failed to serialize filter options: {}", e))?;
        println!("{}", json);
        return Ok(());
    }

    let products = &response.products;

    if json_output {
        let json = serde_json::to_string_pretty(products)
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

fn cmd_stock(
    part_number: &str,
    client_id: Option<&str>,
    client_secret: Option<&str>,
    json_output: bool,
    sandbox: bool,
) -> Result<(), String> {
    let (client_id, client_secret) = get_credentials(client_id, client_secret)?;
    let access_token = get_access_token(&client_id, &client_secret, sandbox)?;

    let product = get_part_by_number(&client_id, &access_token, part_number, sandbox)?;

    let mpn = product
        .manufacturer_part_number
        .clone()
        .or_else(|| product.digi_key_part_number.clone())
        .unwrap_or_else(|| part_number.to_string());

    let price_breaks: Vec<StockPriceBreak> = product
        .standard_pricing
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|pb| {
            Some(StockPriceBreak {
                quantity: pb.break_quantity?,
                unit_price: pb.unit_price?,
            })
        })
        .collect();

    let info = StockInfo {
        mpn,
        manufacturer: product.manufacturer.as_ref().and_then(|m| m.name.clone()),
        distributor: "digikey",
        distributor_pn: product.digi_key_part_number.clone(),
        lifecycle_status: product.part_status.clone(),
        stock: product.quantity_available.map(|q| q as i64),
        lead_time: None,
        moq: product.minimum_order_quantity,
        order_multiple: None,
        currency: "USD".to_string(),
        price_breaks,
    };

    if json_output {
        let json = serde_json::to_string_pretty(&info)
            .map_err(|e| format!("Failed to serialize stock info: {}", e))?;
        println!("{}", json);
    } else {
        let mfr_display = info
            .manufacturer
            .as_deref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        println!("{}{}", info.mpn, mfr_display);

        let dist_pn = info
            .distributor_pn
            .as_deref()
            .map(|p| format!(" ({})", p))
            .unwrap_or_default();
        println!("  Distributor: DigiKey{}", dist_pn);

        if let Some(ref status) = info.lifecycle_status {
            println!("  Status: {}", status);
        }

        match info.stock {
            Some(s) => println!("  Stock: {}", format_number(s)),
            None => println!("  Stock: Unknown"),
        }

        let moq_str = info.moq.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        println!("  MOQ: {}", moq_str);

        if !info.price_breaks.is_empty() {
            println!("  Pricing:");
            let max_qty_width = info
                .price_breaks
                .iter()
                .map(|pb| format!("{}+", pb.quantity).len())
                .max()
                .unwrap_or(3);
            for pb in &info.price_breaks {
                let qty_label = format!("{}+", pb.quantity);
                println!(
                    "    {:>width$} : ${:.2}",
                    qty_label,
                    pb.unit_price,
                    width = max_qty_width
                );
            }
        }
    }

    Ok(())
}

fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn search_by_keyword(
    client_id: &str,
    access_token: &str,
    keyword: &str,
    limit: usize,
    sandbox: bool,
    filter_options_request: Option<FilterOptionsRequest>,
    sort_options: Option<SortOptions>,
) -> Result<SearchResponse, String> {
    let base_url = if sandbox { DIGIKEY_API_BASE_SANDBOX } else { DIGIKEY_API_BASE };
    let url = format!("{}/products/v4/search/keyword", base_url);

    let request = KeywordSearchRequest {
        keywords: keyword.to_string(),
        limit: Some(limit),
        offset: Some(0),
        filter_options_request,
        sort_options,
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

    Ok(response)
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
