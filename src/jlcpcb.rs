//! JLCPCB/LCSC component search integration.
//!
//! Provides CLI commands for searching JLCPCB's SMT parts library and retrieving
//! component details including assembly category (basic/preferred/extended),
//! pricing, and stock levels. No API key required.

use clap::Subcommand;
use serde::{Deserialize, Serialize};

const SEARCH_URL: &str =
    "https://jlcpcb.com/api/overseas-pcb-order/v1/shoppingCart/smtGood/selectSmtComponentList/v2";
const DETAIL_URL: &str =
    "https://cart.jlcpcb.com/shoppingCart/smtGood/getComponentDetail";

/// JLCPCB subcommands.
#[derive(Subcommand, Debug)]
pub enum JlcpcbSubcommand {
    /// Search for JLCPCB/LCSC parts by keyword
    Search {
        /// Search query (part number, keyword, or description)
        query: String,

        /// Maximum number of results to return (max 100)
        #[arg(long, short, default_value = "10")]
        limit: usize,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Filter by manufacturer name
        #[arg(long, short)]
        manufacturer: Option<String>,

        /// Filter by package/footprint (e.g. 0402, SOT-23)
        #[arg(long, short)]
        package: Option<String>,

        /// Show only basic library parts
        #[arg(long)]
        basic_only: bool,

        /// Show only in-stock parts
        #[arg(long)]
        in_stock: bool,
    },

    /// Get detailed information about a specific LCSC part
    Part {
        /// LCSC part number (e.g. C14663)
        lcsc_part_number: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Quick stock and pricing check for a part
    Stock {
        /// LCSC part number or manufacturer part number
        part_number: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    jlcpcb_category: Option<String>,
}

#[derive(Serialize)]
struct StockPriceBreak {
    quantity: i32,
    unit_price: f64,
}

// --- Output types (what we serialize for --json) ---

#[derive(Serialize, Debug)]
pub struct JlcpcbPart {
    pub lcsc_part_number: String,
    pub manufacturer_part_number: Option<String>,
    pub manufacturer: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub package: Option<String>,
    pub stock: Option<i64>,
    pub price_breaks: Vec<PriceBreak>,
    pub datasheet_url: Option<String>,
    pub product_url: Option<String>,
    pub first_category: Option<String>,
    pub second_category: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct PriceBreak {
    pub quantity: i32,
    pub price_usd: f64,
}

#[derive(Serialize, Debug)]
pub struct JlcpcbPartDetail {
    pub lcsc_part_number: String,
    pub manufacturer_part_number: Option<String>,
    pub manufacturer: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub package: Option<String>,
    pub stock: Option<i64>,
    pub price_breaks: Vec<PriceBreak>,
    pub datasheet_url: Option<String>,
    pub product_url: Option<String>,
    pub first_category: Option<String>,
    pub second_category: Option<String>,
    pub assembly_process: Option<String>,
    pub minimum_order: Option<i32>,
    pub attributes: Vec<PartAttribute>,
}

#[derive(Serialize, Debug)]
pub struct PartAttribute {
    pub name: String,
    pub value: String,
}

// --- JLCPCB API wire types (deserialization only) ---

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    code: i32,
    message: Option<String>,
    data: Option<T>,
}

// Search endpoint types

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SearchData {
    component_page_info: Option<PageInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    list: Option<Vec<SearchComponent>>,
    #[allow(dead_code)]
    total: Option<i64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SearchComponent {
    component_code: Option<String>,
    component_model_en: Option<String>,
    component_brand_en: Option<String>,
    describe: Option<String>,
    component_library_type: Option<String>,
    component_specification_en: Option<String>,
    stock_count: Option<i64>,
    component_prices: Option<Vec<ApiPriceBreak>>,
    data_manual_url: Option<String>,
    lcsc_goods_url: Option<String>,
    first_sort_name: Option<String>,
    second_sort_name: Option<String>,
    preferred_component_flag: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiPriceBreak {
    start_number: Option<i32>,
    product_price: Option<f64>,
}

// Detail endpoint types

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct DetailComponent {
    component_code: Option<String>,
    component_model_en: Option<String>,
    component_brand_en: Option<String>,
    describe: Option<String>,
    component_library_type: Option<String>,
    component_specification_en: Option<String>,
    stock_count: Option<i64>,
    prices: Option<Vec<DetailPriceBreak>>,
    data_manual_url: Option<String>,
    lcsc_goods_url: Option<String>,
    first_sort_name: Option<String>,
    second_sort_name: Option<String>,
    assembly_process: Option<String>,
    min_purchase_num: Option<i32>,
    attributes: Option<Vec<ApiAttribute>>,
    preferred_component_flag: Option<bool>,
    first_type_name_en: Option<String>,
    second_type_name_en: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DetailPriceBreak {
    start_number: Option<i32>,
    product_price: Option<f64>,
    #[allow(dead_code)]
    deleted: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct ApiAttribute {
    attribute_name_en: Option<String>,
    attribute_value_name: Option<String>,
}

// --- Command execution ---

pub fn execute(command: JlcpcbSubcommand) -> Result<(), String> {
    match command {
        JlcpcbSubcommand::Search {
            query,
            limit,
            json,
            manufacturer,
            package,
            basic_only,
            in_stock,
        } => cmd_search(&query, limit, json, manufacturer.as_deref(), package.as_deref(), basic_only, in_stock),
        JlcpcbSubcommand::Part {
            lcsc_part_number,
            json,
        } => cmd_part(&lcsc_part_number, json),
        JlcpcbSubcommand::Stock { part_number, json } => cmd_stock(&part_number, json),
    }
}

fn cmd_search(
    query: &str,
    limit: usize,
    json_output: bool,
    manufacturer: Option<&str>,
    package: Option<&str>,
    basic_only: bool,
    in_stock: bool,
) -> Result<(), String> {
    let limit = limit.min(100);
    let parts = jlcpcb_search(query, limit, manufacturer, package, basic_only, in_stock)?;

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

        for (i, part) in parts.iter().enumerate() {
            println!("{}. {}", i + 1, format_part_summary(part));
            println!();
        }
    }

    Ok(())
}

fn cmd_part(part_number: &str, json_output: bool) -> Result<(), String> {
    // If it looks like an LCSC part number (C followed by digits), use the detail endpoint directly.
    // Otherwise, search by MPN and use the first result.
    let lcsc_pn = if is_lcsc_part_number(part_number) {
        part_number.to_string()
    } else {
        let results = jlcpcb_search(part_number, 5, None, None, false, false)?;
        let first = results.into_iter().next().ok_or_else(|| {
            format!("No JLCPCB/LCSC part found for: {}", part_number)
        })?;
        eprintln!(
            "Resolved {} -> {} ({})",
            part_number,
            first.lcsc_part_number,
            first.manufacturer_part_number.as_deref().unwrap_or("?")
        );
        first.lcsc_part_number
    };

    let part = jlcpcb_part_detail(&lcsc_pn)?;

    if json_output {
        let json = serde_json::to_string_pretty(&part)
            .map_err(|e| format!("Failed to serialize part: {}", e))?;
        println!("{}", json);
    } else {
        print_part_details(&part);
    }

    Ok(())
}

fn cmd_stock(part_number: &str, json_output: bool) -> Result<(), String> {
    let part = if is_lcsc_part_number(part_number) {
        let detail = jlcpcb_part_detail(part_number)?;
        let price_breaks: Vec<StockPriceBreak> = detail
            .price_breaks
            .iter()
            .map(|pb| StockPriceBreak {
                quantity: pb.quantity,
                unit_price: pb.price_usd,
            })
            .collect();
        StockInfo {
            mpn: detail
                .manufacturer_part_number
                .clone()
                .unwrap_or_else(|| detail.lcsc_part_number.clone()),
            manufacturer: detail.manufacturer.clone(),
            distributor: "jlcpcb",
            distributor_pn: Some(detail.lcsc_part_number.clone()),
            lifecycle_status: None,
            stock: detail.stock,
            lead_time: None,
            moq: detail.minimum_order,
            order_multiple: None,
            currency: "USD".to_string(),
            price_breaks,
            jlcpcb_category: detail.category.clone(),
        }
    } else {
        let results = jlcpcb_search(part_number, 5, None, None, false, false)?;
        let first = results.into_iter().next().ok_or_else(|| {
            format!("No JLCPCB/LCSC part found for: {}", part_number)
        })?;
        eprintln!(
            "Resolved {} -> {} ({})",
            part_number,
            first.lcsc_part_number,
            first.manufacturer_part_number.as_deref().unwrap_or("?")
        );
        let price_breaks: Vec<StockPriceBreak> = first
            .price_breaks
            .iter()
            .map(|pb| StockPriceBreak {
                quantity: pb.quantity,
                unit_price: pb.price_usd,
            })
            .collect();
        StockInfo {
            mpn: first
                .manufacturer_part_number
                .clone()
                .unwrap_or_else(|| first.lcsc_part_number.clone()),
            manufacturer: first.manufacturer.clone(),
            distributor: "jlcpcb",
            distributor_pn: Some(first.lcsc_part_number.clone()),
            lifecycle_status: None,
            stock: first.stock,
            lead_time: None,
            moq: None,
            order_multiple: None,
            currency: "USD".to_string(),
            price_breaks,
            jlcpcb_category: first.category.clone(),
        }
    };

    if json_output {
        let json = serde_json::to_string_pretty(&part)
            .map_err(|e| format!("Failed to serialize stock info: {}", e))?;
        println!("{}", json);
    } else {
        let mfr_display = part
            .manufacturer
            .as_deref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        println!("{}{}", part.mpn, mfr_display);

        let dist_pn = part
            .distributor_pn
            .as_deref()
            .map(|p| format!(" ({})", p))
            .unwrap_or_default();
        println!("  Distributor: JLCPCB/LCSC{}", dist_pn);

        if let Some(ref category) = part.jlcpcb_category {
            println!("  JLCPCB Category: {}", category);
        }

        match part.stock {
            Some(s) => println!("  Stock: {}", format_number(s)),
            None => println!("  Stock: Unknown"),
        }

        let moq_str = part.moq.map(|v| v.to_string()).unwrap_or_else(|| "?".to_string());
        println!("  MOQ: {}", moq_str);

        if !part.price_breaks.is_empty() {
            println!("  Pricing:");
            let max_qty_width = part
                .price_breaks
                .iter()
                .map(|pb| format!("{}+", pb.quantity).len())
                .max()
                .unwrap_or(3);
            for pb in &part.price_breaks {
                let qty_label = format!("{}+", pb.quantity);
                println!(
                    "    {:>width$} : ${:.4}",
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

fn is_lcsc_part_number(s: &str) -> bool {
    s.starts_with('C') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit())
}

// --- API helpers ---

fn jlcpcb_search(
    keyword: &str,
    limit: usize,
    manufacturer: Option<&str>,
    package: Option<&str>,
    basic_only: bool,
    in_stock: bool,
) -> Result<Vec<JlcpcbPart>, String> {
    let mut body = serde_json::json!({
        "currentPage": 1,
        "pageSize": limit,
        "keyword": keyword,
        "searchSource": "search",
        "componentAttributes": []
    });

    let obj = body.as_object_mut().unwrap();
    if let Some(m) = manufacturer {
        obj.insert("componentBrand".to_string(), serde_json::json!(m));
    }
    if let Some(p) = package {
        obj.insert("componentSpecification".to_string(), serde_json::json!(p));
    }
    if basic_only {
        obj.insert("componentLibraryType".to_string(), serde_json::json!("base"));
    }
    if in_stock {
        obj.insert("stockFlag".to_string(), serde_json::json!(true));
    }

    let response: ApiResponse<SearchData> = ureq::post(SEARCH_URL)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .send_json(&body)
        .map_err(|e| format!("JLCPCB search request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse JLCPCB search response: {}", e))?;

    if response.code != 200 {
        return Err(format!(
            "JLCPCB API error (code {}): {}",
            response.code,
            response.message.unwrap_or_default()
        ));
    }

    let components = response
        .data
        .and_then(|d| d.component_page_info)
        .and_then(|p| p.list)
        .unwrap_or_default();

    Ok(components.into_iter().map(|c| convert_search_component(c)).collect())
}

fn jlcpcb_part_detail(lcsc_part_number: &str) -> Result<JlcpcbPartDetail, String> {
    let url = format!("{}?componentCode={}", DETAIL_URL, lcsc_part_number);

    let response: ApiResponse<DetailComponent> = ureq::get(&url)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("JLCPCB part detail request failed: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse JLCPCB part detail response: {}", e))?;

    if response.code != 200 {
        return Err(format!(
            "JLCPCB API error (code {}): {}",
            response.code,
            response.message.unwrap_or_default()
        ));
    }

    let detail = response
        .data
        .ok_or_else(|| format!("Part not found: {}", lcsc_part_number))?;

    Ok(convert_detail_component(detail))
}

// --- Conversion helpers ---

fn normalize_category(library_type: Option<&str>, preferred: Option<bool>) -> Option<String> {
    match library_type {
        Some("base") => Some("basic".to_string()),
        Some("expand") => {
            if preferred == Some(true) {
                Some("preferred".to_string())
            } else {
                Some("extended".to_string())
            }
        }
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

fn convert_price_breaks(prices: &[ApiPriceBreak]) -> Vec<PriceBreak> {
    prices
        .iter()
        .filter_map(|p| {
            Some(PriceBreak {
                quantity: p.start_number?,
                price_usd: p.product_price?,
            })
        })
        .collect()
}

fn convert_detail_price_breaks(prices: &[DetailPriceBreak]) -> Vec<PriceBreak> {
    prices
        .iter()
        .filter(|p| p.deleted != Some(true))
        .filter_map(|p| {
            Some(PriceBreak {
                quantity: p.start_number?,
                price_usd: p.product_price?,
            })
        })
        .collect()
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.is_empty())
}

fn convert_search_component(c: SearchComponent) -> JlcpcbPart {
    let category = normalize_category(
        c.component_library_type.as_deref(),
        c.preferred_component_flag,
    );

    JlcpcbPart {
        lcsc_part_number: c.component_code.unwrap_or_default(),
        manufacturer_part_number: non_empty(c.component_model_en),
        manufacturer: non_empty(c.component_brand_en),
        description: non_empty(c.describe),
        category,
        package: non_empty(c.component_specification_en),
        stock: c.stock_count,
        price_breaks: c.component_prices.map(|p| convert_price_breaks(&p)).unwrap_or_default(),
        datasheet_url: non_empty(c.data_manual_url),
        product_url: non_empty(c.lcsc_goods_url),
        first_category: non_empty(c.first_sort_name),
        second_category: non_empty(c.second_sort_name),
    }
}

fn convert_detail_component(c: DetailComponent) -> JlcpcbPartDetail {
    let category = normalize_category(
        c.component_library_type.as_deref(),
        c.preferred_component_flag,
    );

    let attributes = c
        .attributes
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| {
            Some(PartAttribute {
                name: a.attribute_name_en?,
                value: a.attribute_value_name?,
            })
        })
        .collect();

    JlcpcbPartDetail {
        lcsc_part_number: c.component_code.unwrap_or_default(),
        manufacturer_part_number: non_empty(c.component_model_en),
        manufacturer: non_empty(c.component_brand_en),
        description: non_empty(c.describe),
        category,
        package: non_empty(c.component_specification_en),
        stock: c.stock_count,
        price_breaks: c.prices.map(|p| convert_detail_price_breaks(&p)).unwrap_or_default(),
        datasheet_url: non_empty(c.data_manual_url),
        product_url: non_empty(c.lcsc_goods_url),
        first_category: non_empty(c.first_sort_name).or(non_empty(c.first_type_name_en)),
        second_category: non_empty(c.second_sort_name).or(non_empty(c.second_type_name_en)),
        assembly_process: non_empty(c.assembly_process),
        minimum_order: c.min_purchase_num,
        attributes,
    }
}

// --- Display helpers ---

fn format_part_summary(part: &JlcpcbPart) -> String {
    let mut lines = Vec::new();

    if let Some(ref mpn) = part.manufacturer_part_number {
        if let Some(ref mfr) = part.manufacturer {
            lines.push(format!("{} ({})", mpn, mfr));
        } else {
            lines.push(mpn.clone());
        }
    } else {
        lines.push(format!("LCSC: {}", part.lcsc_part_number));
    }

    lines.push(format!("   LCSC: {}", part.lcsc_part_number));

    if let Some(ref desc) = part.description {
        lines.push(format!("   {}", desc));
    }

    if let Some(ref category) = part.category {
        if let Some(ref pkg) = part.package {
            lines.push(format!("   Category: {} | Package: {}", category, pkg));
        } else {
            lines.push(format!("   Category: {}", category));
        }
    } else if let Some(ref pkg) = part.package {
        lines.push(format!("   Package: {}", pkg));
    }

    if let Some(stock) = part.stock {
        lines.push(format!("   Stock: {}", stock));
    }

    if let Some(first) = part.price_breaks.first() {
        lines.push(format!(
            "   Price: ${:.4} (qty {}+)",
            first.price_usd, first.quantity
        ));
    }

    if part.datasheet_url.is_some() {
        lines.push("   Datasheet: Available".to_string());
    }

    lines.join("\n")
}

fn print_part_details(part: &JlcpcbPartDetail) {
    println!("Part Details");
    println!("============");

    if let Some(ref mpn) = part.manufacturer_part_number {
        println!("Manufacturer Part Number: {}", mpn);
    }
    if let Some(ref mfr) = part.manufacturer {
        println!("Manufacturer: {}", mfr);
    }
    println!("LCSC Part Number: {}", part.lcsc_part_number);
    if let Some(ref desc) = part.description {
        println!("Description: {}", desc);
    }
    if let Some(ref category) = part.category {
        println!("JLCPCB Category: {}", category);
    }
    if let Some(ref pkg) = part.package {
        println!("Package: {}", pkg);
    }
    if let Some(ref process) = part.assembly_process {
        println!("Assembly Process: {}", process);
    }

    if let Some(ref cat1) = part.first_category {
        println!("Category: {}", cat1);
    }
    if let Some(ref cat2) = part.second_category {
        println!("Subcategory: {}", cat2);
    }

    println!();
    println!("Availability");
    println!("------------");
    if let Some(stock) = part.stock {
        println!("In Stock: {}", stock);
    }
    if let Some(min) = part.minimum_order {
        println!("Minimum Order: {}", min);
    }

    if !part.price_breaks.is_empty() {
        println!();
        println!("Pricing");
        println!("-------");
        for pb in &part.price_breaks {
            println!("  {:>6}+ : ${:.4}", pb.quantity, pb.price_usd);
        }
    }

    if !part.attributes.is_empty() {
        println!();
        println!("Attributes");
        println!("----------");
        for attr in part.attributes.iter().take(15) {
            println!("  {}: {}", attr.name, attr.value);
        }
        if part.attributes.len() > 15 {
            println!("  ... and {} more", part.attributes.len() - 15);
        }
    }

    println!();
    println!("Links");
    println!("-----");
    if let Some(ref url) = part.product_url {
        println!("Product Page: {}", url);
    }
    if let Some(ref url) = part.datasheet_url {
        if !url.is_empty() {
            println!("Datasheet: {}", url);
        } else {
            println!("Datasheet: Not available");
        }
    } else {
        println!("Datasheet: Not available");
    }
}
