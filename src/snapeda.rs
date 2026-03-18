//! SnapEDA/SnapMagic CAD library integration.

use std::io::{Cursor, Read as _, Write as _};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://www.snapeda.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

// --- SnapEDA file-based cache ---

mod cache {
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Serialize, Deserialize)]
    struct CacheEntry {
        timestamp: u64,
        ttl_secs: u64,
        data: String,
    }

    impl CacheEntry {
        fn is_expired(&self) -> bool {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now >= self.timestamp + self.ttl_secs
        }
    }

    fn cache_root() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join("datasheet-cli").join("snapeda"))
    }

    fn sanitize_key(key: &str) -> String {
        key.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }

    pub fn cache_get(category: &str, key: &str) -> Option<String> {
        let root = cache_root()?;
        let path = root.join(category).join(format!("{}.json", sanitize_key(key)));
        let content = fs::read_to_string(&path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;
        if entry.is_expired() {
            return None;
        }
        Some(entry.data)
    }

    pub fn cache_set(category: &str, key: &str, value: &str, ttl_secs: u64) {
        let root = match cache_root() {
            Some(r) => r,
            None => return,
        };
        let dir = root.join(category);
        if fs::create_dir_all(&dir).is_err() {
            return;
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = CacheEntry {
            timestamp,
            ttl_secs,
            data: value.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let path = dir.join(format!("{}.json", sanitize_key(key)));
            let _ = fs::write(path, json);
        }
    }
}

use cache::{cache_get, cache_set};

// --- Subcommands ---

/// SnapEDA subcommands.
#[derive(Subcommand, Debug)]
pub enum SnapedaSubcommand {
    /// Search for parts on SnapEDA by name or keyword
    Search {
        /// Search query (part number, keyword, or description)
        query: String,
        /// Maximum number of results to return
        #[arg(long, short, default_value = "10")]
        limit: usize,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Get symbol, footprint, and pin-to-pad mapping for a SnapEDA part
    Part {
        /// SnapEDA unipart ID, part number, or URL
        part: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Pretty-print JSON output
        #[arg(long, short)]
        formatted: bool,
        /// Write output to file
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Get only schematic symbol / pinout data
    Symbol {
        /// SnapEDA unipart ID, part number, or URL
        part: String,
        #[arg(long)]
        json: bool,
        #[arg(long, short)]
        formatted: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Get only PCB footprint data
    Footprint {
        /// SnapEDA unipart ID, part number, or URL
        part: String,
        #[arg(long)]
        json: bool,
        #[arg(long, short)]
        formatted: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Log in to SnapEDA to enable authenticated downloads
    Login {
        /// SnapEDA account email/username
        #[arg(long)]
        username: Option<String>,
        /// SnapEDA account password (will prompt if not provided)
        #[arg(long)]
        password: Option<String>,
    },
    /// Download CAD files from SnapEDA (requires login)
    Download {
        /// SnapEDA unipart ID, part number, or URL
        part: String,
        /// Download format: altium_native (default), eagle, kicad, kicad_mod, kicad_modv6
        #[arg(long, default_value = "altium_native")]
        format: String,
        /// Write output to file
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

// --- API wire types (deserialization only) ---

#[derive(Deserialize, Debug)]
struct UnipartInfo {
    part_id: u64,
    modelname: Option<String>,
    manufacturer: Option<String>,
    part_description: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FootprintInfo {
    name: Option<String>,
}

// Search endpoint types

#[derive(Deserialize, Debug)]
struct SearchResponse {
    #[allow(dead_code)]
    query_type: Option<String>,
    results: Vec<SearchResult>,
}

#[derive(Deserialize, Serialize, Debug)]
struct SearchResult {
    unipart_id: String,
    part_number: String,
    manufacturer: String,
    #[serde(default)]
    has_symbol: serde_json::Value,
    #[serde(default)]
    has_footprint: serde_json::Value,
    #[serde(default)]
    has_3d: serde_json::Value,
    #[serde(default)]
    has_datasheet: serde_json::Value,
    short_description: Option<String>,
    #[serde(default)]
    package_type: Option<String>,
    #[serde(default)]
    te_param: Option<TeParam>,
}

#[derive(Deserialize, Serialize, Debug)]
struct TeParam {
    #[serde(default)]
    part_images: Option<PartImages>,
}

#[derive(Deserialize, Serialize, Debug)]
struct PartImages {
    symbol_id: Option<String>,
    footprint_id: Option<String>,
    #[serde(rename = "3dmodel_id")]
    model_3d_id: Option<String>,
}

// --- Internal Eagle parsed types ---

struct EagleSmd {
    name: String,
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    layer: u32,
    cream: bool,
}

struct EagleThroughHolePad {
    name: String,
    x: f64,
    y: f64,
    drill: f64,
    diameter: Option<f64>,
    shape: Option<String>,
}

enum EaglePad {
    Smd(EagleSmd),
    ThroughHole(EagleThroughHolePad),
}

struct EagleWire {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    #[allow(dead_code)]
    width: f64,
    layer: u32,
}

struct EaglePin {
    name: String,
    #[allow(dead_code)]
    x: f64,
    #[allow(dead_code)]
    y: f64,
    direction: String,
    rotation: Option<String>,
    #[allow(dead_code)]
    length: String,
}

struct EagleConnect {
    pin: String,
    pad: String,
}

struct EaglePackage {
    name: String,
    pads: Vec<EaglePad>,
    wires: Vec<EagleWire>,
}

struct EagleSymbol {
    name: String,
    pins: Vec<EaglePin>,
}

struct EagleDeviceSet {
    name: String,
    prefix: Option<String>,
    package_name: Option<String>,
    connects: Vec<EagleConnect>,
}

struct EagleLibrary {
    packages: Vec<EaglePackage>,
    symbols: Vec<EagleSymbol>,
    devicesets: Vec<EagleDeviceSet>,
}

// --- Output JSON types ---

#[derive(Serialize, Debug)]
struct SnapedaOutput {
    pinout: PinoutData,
    footprint: FootprintData,
    pin_to_pad_map: Vec<PinPadMapping>,
    snapeda_source: SnapedaSource,
}

#[derive(Serialize, Debug)]
struct SnapedaSource {
    unipart_id: String,
    model_id: String,
    ipc_package: Option<String>,
}

#[derive(Serialize, Debug)]
struct PinPadMapping {
    pin_name: String,
    pad_number: String,
}

#[derive(Serialize, Debug)]
struct PinoutData {
    part_details: PinoutPartDetails,
    packages: Vec<PinoutPackage>,
}

#[derive(Serialize, Debug)]
struct PinoutPartDetails {
    part_number: String,
    description: Option<String>,
    datasheet_revision: Option<String>,
}

#[derive(Serialize, Debug)]
struct PinoutPackage {
    package_name: String,
    pins: Vec<PinoutPin>,
}

#[derive(Serialize, Debug)]
struct PinoutPin {
    pin_number: String,
    pin_name: String,
    electrical_type: String,
    functional_group: Option<String>,
    description: Option<String>,
    alternate_functions: Vec<String>,
}

#[derive(Serialize, Debug)]
struct FootprintData {
    part_details: FootprintPartDetails,
    packages: Vec<FootprintPackage>,
}

#[derive(Serialize, Debug)]
struct FootprintPartDetails {
    part_number: String,
    datasheet_revision: Option<String>,
}

#[derive(Serialize, Debug)]
struct FootprintPackage {
    package_code: String,
    package_name: String,
    pin_count: usize,
    pad_data_source: String,
    pads: Vec<FootprintPad>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thermal_pad: Option<ThermalPad>,
    #[serde(skip_serializing_if = "Option::is_none")]
    courtyard: Option<Courtyard>,
    component_dimensions: ComponentDimensions,
}

#[derive(Serialize, Debug)]
struct FootprintPad {
    number: u32,
    shape: String,
    x: String,
    y: String,
    width: String,
    height: String,
    layers: String,
}

#[derive(Serialize, Debug)]
struct ThermalPad {
    shape: String,
    x: String,
    y: String,
    width: String,
    height: String,
}

#[derive(Serialize, Debug)]
struct Courtyard {
    width: String,
    height: String,
    line_width: String,
}

#[derive(Serialize, Debug)]
struct ComponentDimensions {
    body_length: Option<String>,
    body_width: Option<String>,
    body_height: Option<String>,
    lead_pitch: Option<String>,
    lead_length: Option<String>,
    lead_span: Option<String>,
    lead_width: Option<String>,
    pin_1_indicator: Option<String>,
}

// --- Command execution ---

pub fn execute(command: SnapedaSubcommand) -> Result<(), String> {
    match command {
        SnapedaSubcommand::Search { query, limit, json } => cmd_search(&query, limit, json),
        SnapedaSubcommand::Part {
            part,
            json,
            formatted,
            out,
        } => cmd_part(&part, json, formatted, out),
        SnapedaSubcommand::Symbol {
            part,
            json,
            formatted,
            out,
        } => cmd_symbol(&part, json, formatted, out),
        SnapedaSubcommand::Footprint {
            part,
            json,
            formatted,
            out,
        } => cmd_footprint(&part, json, formatted, out),
        SnapedaSubcommand::Login { username, password } => cmd_login(username, password),
        SnapedaSubcommand::Download { part, format, out } => cmd_download(&part, &format, out),
    }
}

fn cmd_search(query: &str, limit: usize, json_output: bool) -> Result<(), String> {
    let results = snapeda_search(query)?;
    let results: Vec<&SearchResult> = results.iter().take(limit).collect();

    if json_output {
        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| format!("Failed to serialize results: {}", e))?;
        println!("{}", json);
    } else {
        if results.is_empty() {
            println!("No parts found for query: {}", query);
            return Ok(());
        }

        println!("Found {} part(s):\n", results.len());

        for (i, r) in results.iter().enumerate() {
            let sym = if is_truthy(&r.has_symbol) { "sym" } else { "   " };
            let fp = if is_truthy(&r.has_footprint) { "fp" } else { "  " };
            let model = if is_truthy(&r.has_3d) { "3d" } else { "  " };
            let desc = r.short_description.as_deref().unwrap_or("");
            let desc_truncated = if desc.len() > 70 { &desc[..70] } else { desc };

            println!(
                "{}. {} ({})",
                i + 1,
                r.part_number,
                r.manufacturer
            );
            println!("   unipart_id: {}  [{}|{}|{}]", r.unipart_id, sym, fp, model);
            if !desc_truncated.is_empty() {
                println!("   {}", desc_truncated);
            }
            println!();
        }
    }

    Ok(())
}

/// Resolved part identifiers from search or direct lookup.
struct ResolvedPart {
    unipart_id: String,
    model_id: u64,
    alt_model_id: Option<u64>,
    part_name: String,
    manufacturer: Option<String>,
    description: Option<String>,
}

/// Resolve part argument and discover the correct Eagle model_id.
/// Strategy:
/// 1. If numeric, try get_part_for_unipart first (fast path)
/// 2. Otherwise, search via search_local_internal (gets te_param.part_images.symbol_id)
/// 3. For search results, prefer te_param symbol_id over get_part_for_unipart model_id
///    because get_part_for_unipart sometimes returns a non-Eagle format model
fn resolve_and_fetch(part: &str) -> Result<ResolvedPart, String> {
    // Direct numeric unipart_id
    if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
        let uid = part.to_string();
        // First try search to get the correct model_id from te_param
        if let Ok(results) = snapeda_search(&uid) {
            if let Some(r) = results.iter().find(|r| r.unipart_id == uid) {
                if let Some(mid) = extract_model_id_from_search(r) {
                    eprintln!("Resolved model_id {} from search for unipart_id {}", mid, uid);
                    thread::sleep(Duration::from_secs(1));
                    let info = fetch_unipart_info(&uid)?;
                    let alt_model_id = if info.part_id != mid { Some(info.part_id) } else { None };
                    return Ok(ResolvedPart {
                        unipart_id: uid,
                        model_id: mid,
                        alt_model_id,
                        part_name: r.part_number.clone(),
                        manufacturer: Some(r.manufacturer.clone()),
                        description: r.short_description.clone(),
                    });
                }
            }
        }
        // Fallback: use get_part_for_unipart
        thread::sleep(Duration::from_secs(1));
        let info = fetch_unipart_info(&uid)?;
        return Ok(ResolvedPart {
            unipart_id: uid,
            model_id: info.part_id,
            alt_model_id: None,
            part_name: info.modelname.unwrap_or_default(),
            manufacturer: info.manufacturer.clone(),
            description: info.part_description.clone(),
        });
    }

    // URL or part name → search
    let (query, mfr_filter) = if part.starts_with("http://") || part.starts_with("https://") {
        parse_url_components(part)?
    } else {
        (part.to_string(), None)
    };

    eprintln!("Searching SnapEDA for: {}", query);
    let results = snapeda_search(&query)?;

    if results.is_empty() {
        return Err(format!("No SnapEDA results found for: {}", query));
    }

    // Find best match
    let matched = if let Some(ref mfr) = mfr_filter {
        let mfr_lower = mfr.to_lowercase();
        results.iter()
            .find(|r| r.part_number == query && r.manufacturer.to_lowercase() == mfr_lower)
            .or_else(|| results.iter().find(|r| r.part_number == query))
            .or(results.first())
    } else {
        results.iter()
            .find(|r| r.part_number.eq_ignore_ascii_case(&query))
            .or(results.first())
    }.unwrap(); // safe: we checked non-empty above

    let uid = matched.unipart_id.clone();
    eprintln!("Resolved: {} by {} (unipart_id: {})", matched.part_number, matched.manufacturer, uid);

    // Get model_id from te_param (preferred) or fallback to get_part_for_unipart
    let (model_id, alt_model_id) = if let Some(mid) = extract_model_id_from_search(matched) {
        eprintln!("Using Eagle model_id {} from search metadata", mid);
        thread::sleep(Duration::from_secs(1));
        let info = fetch_unipart_info(&uid)?;
        let alt = if info.part_id != mid { Some(info.part_id) } else { None };
        (mid, alt)
    } else {
        thread::sleep(Duration::from_secs(1));
        let info = fetch_unipart_info(&uid)?;
        (info.part_id, None)
    };

    Ok(ResolvedPart {
        unipart_id: uid,
        model_id,
        alt_model_id,
        part_name: matched.part_number.clone(),
        manufacturer: Some(matched.manufacturer.clone()),
        description: matched.short_description.clone(),
    })
}

fn extract_model_id_from_search(r: &SearchResult) -> Option<u64> {
    r.te_param.as_ref()
        .and_then(|tp| tp.part_images.as_ref())
        .and_then(|pi| pi.symbol_id.as_ref())
        .and_then(|s| s.parse::<u64>().ok())
}

fn parse_url_components(url: &str) -> Result<(String, Option<String>), String> {
    let path = url
        .split("snapeda.com")
        .nth(1)
        .or_else(|| url.split("snapmagic.com").nth(1))
        .unwrap_or(url);

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() >= 3 && segments[0] == "parts" {
        let part_name = urlencoding::decode(segments[1])
            .unwrap_or_else(|_| segments[1].into())
            .into_owned();
        let manufacturer = urlencoding::decode(segments[2])
            .unwrap_or_else(|_| segments[2].into())
            .into_owned();
        Ok((part_name, Some(manufacturer)))
    } else {
        Err(format!(
            "Could not parse SnapEDA URL: {}\n\
             Expected: https://www.snapeda.com/parts/<part>/<manufacturer>/view-part/",
            url
        ))
    }
}

fn fetch_part_output(resolved: &ResolvedPart) -> Result<SnapedaOutput, String> {
    thread::sleep(Duration::from_secs(1));

    let mut effective_model_id = resolved.model_id;
    let eagle_result = fetch_eagle_xml(resolved.model_id);

    let eagle_result = match (&eagle_result, resolved.alt_model_id) {
        (Err(primary_err), Some(alt_id)) => {
            eprintln!("Primary model_id {} failed: {}", resolved.model_id, primary_err);
            eprintln!("Trying alternate model_id {}...", alt_id);
            thread::sleep(Duration::from_secs(1));
            let alt_result = fetch_eagle_xml(alt_id);
            if alt_result.is_ok() {
                effective_model_id = alt_id;
            }
            alt_result
        }
        _ => eagle_result,
    };

    let library = match eagle_result {
        Ok(ref xml_str) => parse_eagle_xml(xml_str)?,
        Err(ref eagle_err) => {
            eprintln!("Eagle XML API unavailable: {}", eagle_err);

            if let Some((session_id, csrf)) = get_session_cookies() {
                // Fallback 1: download Eagle format via authenticated API
                // (returns full symbol + footprint + deviceset data)
                eprintln!("Trying authenticated Eagle download...");
                match authenticated_download_text(resolved, &session_id, &csrf, "eagle", ".lbr") {
                    Ok(eagle_xml) => {
                        eprintln!("Got Eagle XML via authenticated download");
                        parse_eagle_xml(&eagle_xml)?
                    }
                    Err(eagle_dl_err) => {
                        eprintln!("Eagle download failed: {}", eagle_dl_err);
                        // Fallback 2: kicad_mod (footprint-only, no symbol data)
                        eprintln!("Trying kicad_mod download as last resort...");
                        match authenticated_download_text(
                            resolved, &session_id, &csrf, "kicad_mod", ".kicad_mod"
                        ) {
                            Ok(kicad_content) => {
                                parse_kicad_mod(&kicad_content, &resolved.part_name)?
                            }
                            Err(kicad_err) => {
                                return Err(format!(
                                    "{}\nEagle download: {}\nkicad_mod download: {}",
                                    eagle_err, eagle_dl_err, kicad_err
                                ));
                            }
                        }
                    }
                }
            } else {
                eprintln!("Not logged in — cannot try authenticated download fallback.");
                eprintln!("Run 'datasheet snapeda login' to enable fallback downloads.");
                return Err(eagle_err.clone());
            }
        }
    };

    thread::sleep(Duration::from_secs(1));
    let fp_info = fetch_footprint_info(&resolved.unipart_id, effective_model_id)?;

    let info = UnipartInfo {
        part_id: effective_model_id,
        modelname: Some(resolved.part_name.clone()),
        manufacturer: resolved.manufacturer.clone(),
        part_description: resolved.description.clone(),
    };

    build_output(
        &resolved.unipart_id,
        &effective_model_id.to_string(),
        &info,
        &fp_info,
        &library,
    )
}

fn cmd_part(
    part: &str,
    json_output: bool,
    formatted: bool,
    out: Option<PathBuf>,
) -> Result<(), String> {
    let resolved = resolve_and_fetch(part)?;
    let output = fetch_part_output(&resolved)?;
    emit_output(json_output, formatted, out, &output)
}

fn cmd_symbol(
    part: &str,
    json_output: bool,
    formatted: bool,
    out: Option<PathBuf>,
) -> Result<(), String> {
    let resolved = resolve_and_fetch(part)?;
    let output = fetch_part_output(&resolved)?;

    let text = if json_output {
        serialize_json(&output.pinout, formatted)?
    } else {
        format_pinout_human(&output)
    };

    write_output(&text, out)
}

fn cmd_footprint(
    part: &str,
    json_output: bool,
    formatted: bool,
    out: Option<PathBuf>,
) -> Result<(), String> {
    let resolved = resolve_and_fetch(part)?;
    let output = fetch_part_output(&resolved)?;

    let text = if json_output {
        serialize_json(&output.footprint, formatted)?
    } else {
        format_footprint_human(&output)
    };

    write_output(&text, out)
}

fn mm(v: f64) -> String {
    // Format with up to 3 decimal places, trimming trailing zeros
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{}mm", s)
}

fn is_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) > 0,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::String(s) => s == "1" || s == "true",
        _ => false,
    }
}

// (Part resolution is handled by resolve_and_fetch above)

// --- API helpers ---

fn fetch_unipart_info(unipart_id: &str) -> Result<UnipartInfo, String> {
    if let Some(cached) = cache_get("unipart", unipart_id) {
        eprintln!("[CACHE] Using cached unipart info for: {}", unipart_id);
        let info: UnipartInfo = serde_json::from_str(&cached)
            .map_err(|e| format!("Failed to parse cached unipart info: {}", e))?;
        return Ok(info);
    }

    let url = format!("{}/api/get_part_for_unipart/{}", BASE_URL, unipart_id);

    let raw = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("Failed to fetch unipart info: {}", e))?
        .into_string()
        .map_err(|e| format!("Failed to read unipart info response: {}", e))?;

    let info: UnipartInfo = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse unipart info response: {}", e))?;

    cache_set("unipart", unipart_id, &raw, 604800);

    Ok(info)
}

fn fetch_eagle_xml(model_id: u64) -> Result<String, String> {
    let key = model_id.to_string();

    if let Some(cached) = cache_get("eagle_xml", &key) {
        eprintln!("[CACHE] Using cached Eagle XML for model_id: {}", model_id);
        return Ok(cached);
    }

    let url = format!("{}/api/get_html_5/{}", BASE_URL, model_id);

    // Retry up to 3 times — this endpoint can return empty intermittently
    for attempt in 1..=3 {
        let xml = ureq::get(&url)
            .set("User-Agent", USER_AGENT)
            .call()
            .map_err(|e| format!("Failed to fetch Eagle XML: {}", e))?
            .into_string()
            .map_err(|e| format!("Failed to read Eagle XML response: {}", e))?;

        if !xml.trim().is_empty() {
            cache_set("eagle_xml", &key, &xml, 604800);
            return Ok(xml);
        }

        if attempt < 3 {
            eprintln!("Eagle XML empty for model_id {}, retrying ({}/3)...", model_id, attempt);
            thread::sleep(Duration::from_secs(2));
        }
    }

    Err(format!(
        "No Eagle XML data available for model_id {}.\n\
         This part may only have native-format CAD files on SnapEDA.\n\
         Try using 'datasheet snapeda search' to find an alternative model.",
        model_id
    ))
}

fn fetch_footprint_info(unipart_id: &str, model_id: u64) -> Result<FootprintInfo, String> {
    let url = format!(
        "{}/parts/snapdata/footprint-info/{}/{}/json",
        BASE_URL, unipart_id, model_id
    );

    let info: FootprintInfo = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("Failed to fetch footprint info: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse footprint info response: {}", e))?;

    Ok(info)
}

fn acquire_csrf_token() -> Result<String, String> {
    if let Some(token) = cache_get("csrf", "token") {
        eprintln!("[CACHE] Using cached SnapEDA CSRF token");
        return Ok(token);
    }

    let response = ureq::get(&format!("{}/account/login/", BASE_URL))
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Failed to fetch SnapEDA login page for CSRF token: {}", e))?;

    // Extract csrftoken from Set-Cookie header
    let cookie_header = response.header("Set-Cookie").unwrap_or("");
    if let Some(start) = cookie_header.find("csrftoken=") {
        let after = &cookie_header[start + 10..];
        let token: String = after.chars().take_while(|c| *c != ';' && *c != ' ').collect();
        if !token.is_empty() {
            cache_set("csrf", "token", &token, 1800);
            return Ok(token);
        }
    }

    // ureq might have consumed the body already; try reading all headers
    Err("Could not extract CSRF token from SnapEDA login page.".to_string())
}

/// Extract a named cookie value from a list of Set-Cookie header values.
fn extract_cookie_value(cookies: &[&str], name: &str) -> Option<String> {
    let prefix = format!("{}=", name);
    for cookie_str in cookies {
        if let Some(start) = cookie_str.find(&prefix) {
            let after = &cookie_str[start + prefix.len()..];
            let value: String = after.chars().take_while(|c| *c != ';' && *c != ' ').collect();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn get_session_cookies() -> Option<(String, String)> {
    let session_id = cache_get("session", "sessionid")?;
    let csrf = cache_get("session", "csrftoken")?;
    Some((session_id, csrf))
}

fn cmd_login(username: Option<String>, password: Option<String>) -> Result<(), String> {
    let username = match username {
        Some(u) => u,
        None => {
            eprint!("SnapEDA username/email: ");
            std::io::stderr().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)
                .map_err(|e| format!("Failed to read username: {}", e))?;
            input.trim().to_string()
        }
    };

    let password = match password {
        Some(p) => p,
        None => {
            eprint!("SnapEDA password: ");
            std::io::stderr().flush().ok();
            rpassword::read_password()
                .map_err(|e| format!("Failed to read password: {}", e))?
        }
    };

    // Use ureq (not reqwest) — ureq's TLS fingerprint passes Cloudflare challenges
    // that reqwest gets blocked by.

    // Step 1: Get a fresh CSRF token from the login page
    eprintln!("Fetching login page for CSRF token...");
    let login_url = format!("{}/account/login/", BASE_URL);

    let get_resp = ureq::get(&login_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Failed to fetch login page: {}", e))?;

    // Use the final URL after redirects (e.g. www.snapeda.com → snapeda.com)
    let resolved_login_url = get_resp.get_url().to_string();
    eprintln!("Login URL resolved to: {}", resolved_login_url);

    let get_cookies = get_resp.all("set-cookie");
    let csrf = extract_cookie_value(&get_cookies, "csrftoken")
        .ok_or_else(|| "Could not extract CSRF token from login page".to_string())?;
    // Django also sets an anonymous sessionid on GET — must send it back with the POST
    let anon_session = extract_cookie_value(&get_cookies, "sessionid")
        .unwrap_or_default();

    eprintln!("Got fresh CSRF token, posting login...");

    // Step 2: POST login to the resolved URL (not the original, which may 301).
    // ureq returns Err(Status(code, resp)) for non-2xx, including redirects.
    let body = format!(
        "csrfmiddlewaretoken={}&username={}&password={}",
        urlencoding::encode(&csrf),
        urlencoding::encode(&username),
        urlencoding::encode(&password),
    );

    let cookie_header = if anon_session.is_empty() {
        format!("csrftoken={}", csrf)
    } else {
        format!("csrftoken={}; sessionid={}", csrf, anon_session)
    };

    // Disable redirects so we can capture Set-Cookie from the login response
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .build();

    let response = agent
        .post(&resolved_login_url)
        .set("User-Agent", USER_AGENT)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("Referer", &resolved_login_url)
        .set("Cookie", &cookie_header)
        .send_string(&body);

    // With redirects(0), ureq returns all responses (including 3xx) as Ok.
    // Django returns 301 (to /home/) on successful login with Set-Cookie: sessionid=...
    // On failure it returns 200 (the login page again).
    let resp = match response {
        Ok(resp) if resp.status() == 301 || resp.status() == 302 => {
            eprintln!("Login response: {} redirect", resp.status());
            resp
        }
        Ok(resp) => {
            return Err(format!(
                "Login failed: got HTTP {} — check your credentials.",
                resp.status()
            ));
        }
        Err(ureq::Error::Status(code, _)) => {
            return Err(format!("Login failed with HTTP status: {}", code));
        }
        Err(e) => {
            return Err(format!("Login request failed: {}", e));
        }
    };

    // Extract session cookies from the 302 response
    let cookies = resp.all("set-cookie");
    let session_id = extract_cookie_value(&cookies, "sessionid")
        .ok_or_else(|| {
            "Login redirected but no session cookie received. Credentials may be wrong.".to_string()
        })?;
    let csrf_token = extract_cookie_value(&cookies, "csrftoken")
        .unwrap_or(csrf);

    cache_set("session", "sessionid", &session_id, 604800);
    cache_set("session", "csrftoken", &csrf_token, 604800);

    eprintln!("Successfully logged in as: {}", username);
    Ok(())
}

fn cmd_download(part: &str, format: &str, out: Option<PathBuf>) -> Result<(), String> {
    let resolved = resolve_and_fetch(part)?;

    let (session_id, csrf) = get_session_cookies()
        .ok_or_else(|| "Not logged in. Run 'datasheet snapeda login' first.".to_string())?;

    let url = format!(
        "{}/parts/snapapi/download-component/{}/{}/{}",
        BASE_URL, resolved.model_id, resolved.unipart_id, format
    );

    eprintln!("Requesting download: {}", url);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Cookie", &format!("sessionid={}; csrftoken={}", session_id, csrf))
        .set("Referer", &format!("{}/", BASE_URL))
        .call()
        .map_err(|e| format!("Download request failed: {}", e))?;

    let body = response.into_string()
        .map_err(|e| format!("Failed to read download response: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse download response: {} (body: {})", e, &body[..body.len().min(200)]))?;

    if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
        return Err(format!("SnapEDA download error: {}", error));
    }

    let download_url = json.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("No download URL in response: {}", &body[..body.len().min(500)]))?;

    // Infer output filename from the S3 URL
    let url_filename = download_url
        .split('/')
        .last()
        .and_then(|s| s.split('?').next())
        .unwrap_or("download")
        .to_string();

    eprintln!("Downloading: {}", url_filename);

    let dl_response = ureq::get(download_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Failed to download file: {}", e))?;

    let mut raw_bytes = Vec::new();
    dl_response.into_reader().read_to_end(&mut raw_bytes)
        .map_err(|e| format!("Failed to read download data: {}", e))?;

    eprintln!("Downloaded {} bytes", raw_bytes.len());

    // Check if it's a zip (PK magic) or a raw file (OLE2/other)
    let is_zip = raw_bytes.len() >= 4 && raw_bytes[0] == b'P' && raw_bytes[1] == b'K';

    if is_zip {
        // Extract from zip archive
        let cursor = Cursor::new(&raw_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("Failed to open zip archive: {}", e))?;

        let target_exts: &[&str] = match format {
            "kicad_mod" | "kicad_modv6" => &[".kicad_mod"],
            "kicad" => &[".kicad_sym"],
            "eagle" => &[".lbr"],
            "altium_native" => &[".PcbLib", ".SchLib", ".IntLib"],
            _ => &[],
        };

        let mut all_names = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                if !file.is_dir() {
                    all_names.push(file.name().to_string());
                    eprintln!("  zip entry: {}", file.name());
                }
            }
        }

        let archive_len = archive.len();
        let mut extracted_any = false;
        for i in 0..archive_len {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let name = file.name().to_string();

            let matches = target_exts.is_empty()
                || target_exts.iter().any(|ext| name.ends_with(ext));
            if file.is_dir() || !matches {
                continue;
            }

            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map_err(|e| format!("Failed to read {} from zip: {}", name, e))?;

            let file_basename = std::path::Path::new(&name)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or(name.clone());

            if let Some(ref out_path) = out {
                let target = if out_path.extension().is_some() && archive_len <= 2 {
                    out_path.clone()
                } else {
                    if !out_path.exists() {
                        std::fs::create_dir_all(out_path)
                            .map_err(|e| format!("Failed to create output directory: {}", e))?;
                    }
                    out_path.join(&file_basename)
                };
                std::fs::write(&target, &bytes)
                    .map_err(|e| format!("Failed to write {}: {}", target.display(), e))?;
                eprintln!("Wrote: {}", target.display());
            } else {
                // Try to print text, or report binary size
                match String::from_utf8(bytes) {
                    Ok(text) => println!("{}", text),
                    Err(e) => eprintln!("{}: binary file ({} bytes) — use --out to save",
                        file_basename, e.into_bytes().len()),
                }
            }
            extracted_any = true;
        }

        if !extracted_any {
            return Err(format!(
                "Could not find matching file in archive. Contents: {:?}", all_names
            ));
        }
    } else {
        // Raw file (not zipped) — e.g. Altium .IntLib (OLE2 compound document)
        if let Some(ref out_path) = out {
            let target = if out_path.is_dir() {
                out_path.join(&url_filename)
            } else {
                out_path.clone()
            };
            if let Some(parent) = target.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create output directory: {}", e))?;
                }
            }
            std::fs::write(&target, &raw_bytes)
                .map_err(|e| format!("Failed to write {}: {}", target.display(), e))?;
            eprintln!("Wrote: {} ({} bytes)", target.display(), raw_bytes.len());
        } else {
            eprintln!("{}: binary file ({} bytes) — use --out to save", url_filename, raw_bytes.len());
        }
    }

    Ok(())
}

/// Authenticated download of a specific format, returning the raw bytes of the first
/// file in the zip/archive that matches `target_ext`.
fn authenticated_download_text(
    resolved: &ResolvedPart,
    session_id: &str,
    csrf: &str,
    format: &str,
    target_ext: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/parts/snapapi/download-component/{}/{}/{}",
        BASE_URL, resolved.model_id, resolved.unipart_id, format
    );

    eprintln!("Requesting {} download: {}", format, url);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Cookie", &format!("sessionid={}; csrftoken={}", session_id, csrf))
        .set("Referer", &format!("{}/", BASE_URL))
        .call()
        .map_err(|e| format!("Download request failed: {}", e))?;

    let body = response.into_string()
        .map_err(|e| format!("Failed to read download response: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse download response: {}", e))?;

    if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
        return Err(format!("SnapEDA {} download error: {}", format, error));
    }

    let download_url = json.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("No download URL in response: {}", &body[..body.len().min(500)]))?;

    let dl_response = ureq::get(download_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Failed to download {}: {}", format, e))?;

    let mut raw_bytes = Vec::new();
    dl_response.into_reader().read_to_end(&mut raw_bytes)
        .map_err(|e| format!("Failed to read {} data: {}", format, e))?;

    // Check if it's a zip
    let is_zip = raw_bytes.len() >= 4 && raw_bytes[0] == b'P' && raw_bytes[1] == b'K';

    if is_zip {
        let cursor = Cursor::new(&raw_bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| format!("Failed to open {} zip: {}", format, e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            if file.name().ends_with(target_ext) {
                let mut content = String::new();
                file.read_to_string(&mut content)
                    .map_err(|e| format!("Failed to read {} from zip: {}", file.name(), e))?;
                eprintln!("Extracted {} from zip: {}", format, file.name());
                return Ok(content);
            }
        }

        let names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .collect();
        Err(format!("No {} file found in {} archive. Contents: {:?}", target_ext, format, names))
    } else {
        // Raw text file (not zipped)
        String::from_utf8(raw_bytes)
            .map_err(|_| format!("{} download is not valid UTF-8 text", format))
    }
}

fn parse_kicad_mod(content: &str, part_name: &str) -> Result<EagleLibrary, String> {
    let mut pads: Vec<EaglePad> = Vec::new();
    let mut wires: Vec<EagleWire> = Vec::new();

    let mut search_from = 0;
    while let Some(pad_start) = content[search_from..].find("(pad ") {
        let abs_start = search_from + pad_start;
        let pad_content = &content[abs_start..];
        if let Some(pad_block) = extract_sexp_block(pad_content) {
            let tokens = tokenize_sexp_flat(pad_block);

            if tokens.len() >= 4 {
                let pad_number = tokens[1].trim_matches('"').to_string();
                let pad_type = tokens[2].as_str();
                let pad_shape = tokens[3].as_str();

                let (x, y) = extract_sexp_xy(pad_block, "at").unwrap_or((0.0, 0.0));
                let (w, h) = extract_sexp_xy(pad_block, "size").unwrap_or((0.0, 0.0));

                match pad_type {
                    "smd" | "connect" => {
                        let has_paste = pad_block.contains("F.Paste");
                        pads.push(EaglePad::Smd(EagleSmd {
                            name: pad_number,
                            x,
                            y,
                            dx: w,
                            dy: h,
                            layer: 1,
                            cream: has_paste,
                        }));
                    }
                    "thru_hole" => {
                        let drill = extract_sexp_single_value(pad_block, "drill").unwrap_or(0.0);
                        pads.push(EaglePad::ThroughHole(EagleThroughHolePad {
                            name: pad_number,
                            x,
                            y,
                            drill,
                            diameter: Some(w.max(h)),
                            shape: Some(pad_shape.to_string()),
                        }));
                    }
                    _ => {}
                }
            }

            search_from = abs_start + pad_block.len();
        } else {
            search_from = abs_start + 5;
        }
    }

    let mut cy_search = 0;
    while let Some(line_start) = content[cy_search..].find("(fp_line ") {
        let abs_start = cy_search + line_start;
        let line_content = &content[abs_start..];
        if let Some(block) = extract_sexp_block(line_content) {
            if block.contains("F.CrtYd") || block.contains("CrtYd") {
                let (x1, y1) = extract_sexp_xy(block, "start").unwrap_or((0.0, 0.0));
                let (x2, y2) = extract_sexp_xy(block, "end").unwrap_or((0.0, 0.0));
                let width = extract_sexp_single_value(block, "width")
                    .or_else(|| extract_sexp_single_value(block, "stroke"))
                    .unwrap_or(0.05);
                wires.push(EagleWire { x1, y1, x2, y2, width, layer: 39 });
            }
            cy_search = abs_start + block.len();
        } else {
            cy_search = abs_start + 9;
        }
    }

    let mut rect_search = 0;
    while let Some(rect_start) = content[rect_search..].find("(fp_rect ") {
        let abs_start = rect_search + rect_start;
        let rect_content = &content[abs_start..];
        if let Some(block) = extract_sexp_block(rect_content) {
            if block.contains("F.CrtYd") || block.contains("CrtYd") {
                let (x1, y1) = extract_sexp_xy(block, "start").unwrap_or((0.0, 0.0));
                let (x2, y2) = extract_sexp_xy(block, "end").unwrap_or((0.0, 0.0));
                let width = extract_sexp_single_value(block, "width")
                    .or_else(|| extract_sexp_single_value(block, "stroke"))
                    .unwrap_or(0.05);
                wires.push(EagleWire { x1, y1, x2: x2, y2: y1, width, layer: 39 });
                wires.push(EagleWire { x1: x2, y1, x2: x2, y2, width, layer: 39 });
                wires.push(EagleWire { x1: x2, y1: y2, x2: x1, y2, width, layer: 39 });
                wires.push(EagleWire { x1, y1: y2, x2: x1, y2: y1, width, layer: 39 });
            }
            rect_search = abs_start + block.len();
        } else {
            rect_search = abs_start + 9;
        }
    }

    Ok(EagleLibrary {
        packages: vec![EaglePackage {
            name: part_name.to_string(),
            pads,
            wires,
        }],
        symbols: vec![],
        devicesets: vec![],
    })
}

fn extract_sexp_block(s: &str) -> Option<&str> {
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_quotes = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            '(' if !in_quotes => depth += 1,
            ')' if !in_quotes => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn tokenize_sexp_flat(s: &str) -> Vec<String> {
    let inner = s.trim();
    let inner = if inner.starts_with('(') && inner.ends_with(')') {
        &inner[1..inner.len() - 1]
    } else {
        inner
    };

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut depth = 0i32;

    for c in inner.chars() {
        match c {
            '"' if depth == 0 => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            '(' if !in_quotes => {
                if depth == 0 && !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
                depth += 1;
            }
            ')' if !in_quotes => {
                depth -= 1;
            }
            ' ' | '\t' | '\n' | '\r' if !in_quotes && depth == 0 => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ if depth == 0 => {
                current.push(c);
            }
            _ => {}
        }
    }
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    tokens
}

fn extract_sexp_xy(block: &str, keyword: &str) -> Option<(f64, f64)> {
    let pattern = format!("({} ", keyword);
    let start = block.find(&pattern)?;
    let after = &block[start + pattern.len()..];
    let end = after.find(')')?;
    let coords = &after[..end];
    let parts: Vec<&str> = coords.split_whitespace().collect();
    if parts.len() >= 2 {
        let x = parts[0].parse::<f64>().ok()?;
        let y = parts[1].parse::<f64>().ok()?;
        Some((x, y))
    } else {
        None
    }
}

fn extract_sexp_single_value(block: &str, keyword: &str) -> Option<f64> {
    let pattern = format!("({} ", keyword);
    let start = block.find(&pattern)?;
    let after = &block[start + pattern.len()..];
    let end = after.find(|c: char| c == ')' || c == ' ')?;
    after[..end].trim().parse::<f64>().ok()
}

fn snapeda_search(query: &str) -> Result<Vec<SearchResult>, String> {
    if let Some(cached) = cache_get("search", query) {
        eprintln!("[CACHE] Using cached SnapEDA search results for: {}", query);
        let results: Vec<SearchResult> = serde_json::from_str(&cached)
            .map_err(|e| format!("Failed to parse cached search results: {}", e))?;
        return Ok(results);
    }

    let csrf = acquire_csrf_token()?;

    thread::sleep(Duration::from_secs(1));

    let body = format!("q={}", urlencoding::encode(query));

    let response = ureq::post(&format!("{}/api/v1/search_local_internal", BASE_URL))
        .set("User-Agent", USER_AGENT)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("X-CSRFToken", &csrf)
        .set("X-Requested-With", "XMLHttpRequest")
        .set("Referer", &format!("{}/search/", BASE_URL))
        .set("Cookie", &format!("csrftoken={}", csrf))
        .send_string(&body)
        .map_err(|e| format!("SnapEDA search request failed: {}", e))?;

    let search_resp: SearchResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse SnapEDA search response: {}", e))?;

    if let Ok(serialized) = serde_json::to_string(&search_resp.results) {
        cache_set("search", query, &serialized, 86400);
    }

    Ok(search_resp.results)
}

// --- Eagle XML parser ---

fn parse_f64(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

fn parse_u32(s: &str) -> u32 {
    s.parse().unwrap_or(0)
}

fn parse_eagle_xml(xml: &str) -> Result<EagleLibrary, String> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| format!("Failed to parse Eagle XML: {}", e))?;

    let root = doc.root_element();

    // Navigate eagle > drawing > library
    let drawing = root
        .children()
        .find(|n| n.has_tag_name("drawing"))
        .ok_or_else(|| "Eagle XML missing <drawing>".to_string())?;

    let library_node = drawing
        .children()
        .find(|n| n.has_tag_name("library"))
        .ok_or_else(|| "Eagle XML missing <library>".to_string())?;

    let mut packages = Vec::new();
    let mut symbols = Vec::new();
    let mut devicesets = Vec::new();

    for child in library_node.children() {
        if child.has_tag_name("packages") {
            for pkg_node in child.children().filter(|n| n.has_tag_name("package")) {
                let name = pkg_node.attribute("name").unwrap_or("").to_string();
                let mut pads: Vec<EaglePad> = Vec::new();
                let mut wires: Vec<EagleWire> = Vec::new();

                for elem in pkg_node.children() {
                    if elem.has_tag_name("smd") {
                        let pad_name = elem.attribute("name").unwrap_or("").to_string();
                        let x = parse_f64(elem.attribute("x").unwrap_or("0"));
                        let y = parse_f64(elem.attribute("y").unwrap_or("0"));
                        let dx = parse_f64(elem.attribute("dx").unwrap_or("0"));
                        let dy = parse_f64(elem.attribute("dy").unwrap_or("0"));
                        let layer = parse_u32(elem.attribute("layer").unwrap_or("1"));
                        let cream = elem.attribute("cream").map(|v| v != "no").unwrap_or(true);
                        pads.push(EaglePad::Smd(EagleSmd {
                            name: pad_name,
                            x,
                            y,
                            dx,
                            dy,
                            layer,
                            cream,
                        }));
                    } else if elem.has_tag_name("pad") {
                        let pad_name = elem.attribute("name").unwrap_or("").to_string();
                        let x = parse_f64(elem.attribute("x").unwrap_or("0"));
                        let y = parse_f64(elem.attribute("y").unwrap_or("0"));
                        let drill = parse_f64(elem.attribute("drill").unwrap_or("0"));
                        let diameter = elem.attribute("diameter").map(|v| parse_f64(v));
                        let shape = elem.attribute("shape").map(|v| v.to_string());
                        pads.push(EaglePad::ThroughHole(EagleThroughHolePad {
                            name: pad_name,
                            x,
                            y,
                            drill,
                            diameter,
                            shape,
                        }));
                    } else if elem.has_tag_name("wire") {
                        let x1 = parse_f64(elem.attribute("x1").unwrap_or("0"));
                        let y1 = parse_f64(elem.attribute("y1").unwrap_or("0"));
                        let x2 = parse_f64(elem.attribute("x2").unwrap_or("0"));
                        let y2 = parse_f64(elem.attribute("y2").unwrap_or("0"));
                        let width = parse_f64(elem.attribute("width").unwrap_or("0"));
                        let layer = parse_u32(elem.attribute("layer").unwrap_or("0"));
                        wires.push(EagleWire { x1, y1, x2, y2, width, layer });
                    }
                }

                packages.push(EaglePackage { name, pads, wires });
            }
        } else if child.has_tag_name("symbols") {
            for sym_node in child.children().filter(|n| n.has_tag_name("symbol")) {
                let name = sym_node.attribute("name").unwrap_or("").to_string();
                let mut pins: Vec<EaglePin> = Vec::new();

                for elem in sym_node.children().filter(|n| n.has_tag_name("pin")) {
                    let pin_name = elem.attribute("name").unwrap_or("").to_string();
                    let x = parse_f64(elem.attribute("x").unwrap_or("0"));
                    let y = parse_f64(elem.attribute("y").unwrap_or("0"));
                    let direction = elem.attribute("direction").unwrap_or("pas").to_string();
                    let rotation = elem.attribute("rot").map(|v| v.to_string());
                    let length = elem.attribute("length").unwrap_or("short").to_string();
                    pins.push(EaglePin {
                        name: pin_name,
                        x,
                        y,
                        direction,
                        rotation,
                        length,
                    });
                }

                symbols.push(EagleSymbol { name, pins });
            }
        } else if child.has_tag_name("devicesets") {
            for ds_node in child.children().filter(|n| n.has_tag_name("deviceset")) {
                let name = ds_node.attribute("name").unwrap_or("").to_string();
                let prefix = ds_node.attribute("prefix").map(|v| v.to_string());

                let mut package_name: Option<String> = None;
                let mut connects: Vec<EagleConnect> = Vec::new();

                if let Some(devices_node) =
                    ds_node.children().find(|n| n.has_tag_name("devices"))
                {
                    if let Some(device_node) =
                        devices_node.children().find(|n| n.has_tag_name("device"))
                    {
                        package_name =
                            device_node.attribute("package").map(|v| v.to_string());

                        if let Some(connects_node) =
                            device_node.children().find(|n| n.has_tag_name("connects"))
                        {
                            for conn in connects_node
                                .children()
                                .filter(|n| n.has_tag_name("connect"))
                            {
                                let pin = conn.attribute("pin").unwrap_or("").to_string();
                                let pad = conn.attribute("pad").unwrap_or("").to_string();
                                connects.push(EagleConnect { pin, pad });
                            }
                        }
                    }
                }

                devicesets.push(EagleDeviceSet {
                    name,
                    prefix,
                    package_name,
                    connects,
                });
            }
        }
    }

    Ok(EagleLibrary { packages, symbols, devicesets })
}

// --- Electrical type and group mapping ---

fn map_electrical_type(direction: &str, pin_name: &str) -> String {
    match direction {
        "in" => "Input".to_string(),
        "out" => "Output".to_string(),
        "io" => "Bidirectional".to_string(),
        "oc" => "Open Drain".to_string(),
        "pas" => "Passive".to_string(),
        "pwr" | "sup" => {
            let name_upper = pin_name.to_uppercase();
            if name_upper.contains("GND")
                || name_upper.contains("VSS")
                || name_upper.contains("PGND")
                || name_upper.contains("AGND")
            {
                "Ground".to_string()
            } else {
                "Power Input".to_string()
            }
        }
        "hiz" => "Hi-Z".to_string(),
        "nc" => "Passive".to_string(),
        _ => "Passive".to_string(),
    }
}

fn infer_functional_group(pin_name: &str, electrical_type: &str) -> Option<String> {
    let name_upper = pin_name.to_uppercase();
    let name_clean: String = name_upper.chars().filter(|c| c.is_alphanumeric()).collect();

    if name_clean.contains("GND")
        || name_clean.contains("VSS")
        || name_clean.contains("PGND")
        || name_clean.contains("AGND")
    {
        return Some("Ground".to_string());
    }

    if name_clean.contains("VCC")
        || name_clean.contains("VDD")
        || name_clean == "V"
        || name_clean.contains("VIN")
        || name_clean.contains("VOUT")
        || name_clean.contains("BAT")
    {
        return Some("Power".to_string());
    }

    if name_clean.contains("SCL")
        || name_clean.contains("SDA")
        || name_clean.contains("SPI")
        || name_clean.contains("MOSI")
        || name_clean.contains("MISO")
        || name_clean.contains("SCK")
        || name_clean.contains("CS")
        || name_clean.contains("SS")
    {
        return Some("Communication".to_string());
    }

    if (electrical_type == "Input" || electrical_type == "Output")
        && (name_clean.contains("CHRG")
            || name_clean.contains("STDBY")
            || name_clean.contains("STATUS")
            || name_clean.contains("LED"))
    {
        return Some("Status".to_string());
    }

    None
}

// --- Output builder ---

fn build_output(
    unipart_id: &str,
    model_id: &str,
    info: &UnipartInfo,
    fp_info: &FootprintInfo,
    library: &EagleLibrary,
) -> Result<SnapedaOutput, String> {
    let part_number = info.modelname.clone().unwrap_or_else(|| unipart_id.to_string());

    // Use the first deviceset
    let deviceset = library.devicesets.first();
    let symbol_name = deviceset.and_then(|ds| {
        // The symbol name is typically the same as the deviceset name
        // but could differ; we look it up from the gates
        Some(ds.name.clone())
    });

    // Find symbol: prefer exact name match, else first
    let symbol = symbol_name
        .as_deref()
        .and_then(|sname| library.symbols.iter().find(|s| s.name == sname))
        .or_else(|| library.symbols.first());

    // Build pinout
    let mut pins: Vec<PinoutPin> = Vec::new();
    let mut pin_pad_map: Vec<PinPadMapping> = Vec::new();

    if let Some(ds) = deviceset {
        for connect in &ds.connects {
            let eagle_pin = symbol.and_then(|s| s.pins.iter().find(|p| p.name == connect.pin));
            let direction = eagle_pin.map(|p| p.direction.as_str()).unwrap_or("pas");
            let electrical_type = map_electrical_type(direction, &connect.pin);
            let functional_group = infer_functional_group(&connect.pin, &electrical_type);

            pins.push(PinoutPin {
                pin_number: connect.pad.clone(),
                pin_name: connect.pin.clone(),
                electrical_type: electrical_type.clone(),
                functional_group,
                description: None,
                alternate_functions: vec![],
            });

            pin_pad_map.push(PinPadMapping {
                pin_name: connect.pin.clone(),
                pad_number: connect.pad.clone(),
            });
        }
    }

    // Sort pins numerically by pad number
    pins.sort_by(|a, b| {
        let na: u64 = a.pin_number.parse().unwrap_or(u64::MAX);
        let nb: u64 = b.pin_number.parse().unwrap_or(u64::MAX);
        na.cmp(&nb).then(a.pin_number.cmp(&b.pin_number))
    });
    pin_pad_map.sort_by(|a, b| {
        let na: u64 = a.pad_number.parse().unwrap_or(u64::MAX);
        let nb: u64 = b.pad_number.parse().unwrap_or(u64::MAX);
        na.cmp(&nb).then(a.pad_number.cmp(&b.pad_number))
    });

    let package_name_for_pinout = deviceset
        .and_then(|ds| ds.package_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let pinout = PinoutData {
        part_details: PinoutPartDetails {
            part_number: part_number.clone(),
            description: info.part_description.clone(),
            datasheet_revision: None,
        },
        packages: vec![PinoutPackage {
            package_name: package_name_for_pinout.clone(),
            pins,
        }],
    };

    // Build footprint
    let ipc_package = fp_info.name.clone();
    let package_code = ipc_package.clone().unwrap_or_else(|| "Unknown".to_string());

    let eagle_package = deviceset
        .and_then(|ds| ds.package_name.as_deref())
        .and_then(|pname| library.packages.iter().find(|p| p.name == pname))
        .or_else(|| library.packages.first());

    let mut fp_pads: Vec<FootprintPad> = Vec::new();
    let mut thermal_pad: Option<ThermalPad> = None;

    if let Some(pkg) = eagle_package {
        for (idx, pad) in pkg.pads.iter().enumerate() {
            match pad {
                EaglePad::Smd(smd) => {
                    let pad_num: u32 = smd.name.parse().unwrap_or((idx + 1) as u32);
                    if smd.cream {
                        fp_pads.push(FootprintPad {
                            number: pad_num,
                            shape: "rect".to_string(),
                            x: mm(smd.x),
                            y: mm(smd.y),
                            width: mm(smd.dx),
                            height: mm(smd.dy),
                            layers: "F.Cu, F.Paste, F.Mask".to_string(),
                        });
                    } else {
                        // Exposed / thermal pad
                        thermal_pad = Some(ThermalPad {
                            shape: "rect".to_string(),
                            x: mm(smd.x),
                            y: mm(smd.y),
                            width: mm(smd.dx),
                            height: mm(smd.dy),
                        });
                    }
                }
                EaglePad::ThroughHole(thp) => {
                    let pad_num: u32 = thp.name.parse().unwrap_or((idx + 1) as u32);
                    let diameter = thp.diameter.unwrap_or(thp.drill * 1.6);
                    let shape_str = thp.shape.clone().unwrap_or_else(|| "circle".to_string());
                    fp_pads.push(FootprintPad {
                        number: pad_num,
                        shape: shape_str,
                        x: mm(thp.x),
                        y: mm(thp.y),
                        width: mm(diameter),
                        height: mm(diameter),
                        layers: "*.Cu, *.Mask".to_string(),
                    });
                }
            }
        }

        // Sort pads numerically
        fp_pads.sort_by(|a, b| a.number.cmp(&b.number));

        // Courtyard: find wires on layer 39
        let courtyard_wires: Vec<&EagleWire> =
            pkg.wires.iter().filter(|w| w.layer == 39).collect();

        let courtyard = if !courtyard_wires.is_empty() {
            let min_x = courtyard_wires
                .iter()
                .flat_map(|w| [w.x1, w.x2])
                .fold(f64::INFINITY, f64::min);
            let max_x = courtyard_wires
                .iter()
                .flat_map(|w| [w.x1, w.x2])
                .fold(f64::NEG_INFINITY, f64::max);
            let min_y = courtyard_wires
                .iter()
                .flat_map(|w| [w.y1, w.y2])
                .fold(f64::INFINITY, f64::min);
            let max_y = courtyard_wires
                .iter()
                .flat_map(|w| [w.y1, w.y2])
                .fold(f64::NEG_INFINITY, f64::max);

            Some(Courtyard {
                width: mm((max_x - min_x).abs()),
                height: mm((max_y - min_y).abs()),
                line_width: mm(courtyard_wires.first().map(|w| w.width).unwrap_or(0.05)),
            })
        } else {
            None
        };

        let pin_count = fp_pads.len() + if thermal_pad.is_some() { 1 } else { 0 };

        let footprint = FootprintData {
            part_details: FootprintPartDetails {
                part_number: part_number.clone(),
                datasheet_revision: None,
            },
            packages: vec![FootprintPackage {
                package_code: package_code.clone(),
                package_name: eagle_package
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
                pin_count,
                pad_data_source: "SnapEDA".to_string(),
                pads: fp_pads,
                thermal_pad,
                courtyard,
                component_dimensions: ComponentDimensions {
                    body_length: None,
                    body_width: None,
                    body_height: None,
                    lead_pitch: None,
                    lead_length: None,
                    lead_span: None,
                    lead_width: None,
                    pin_1_indicator: None,
                },
            }],
        };

        return Ok(SnapedaOutput {
            pinout,
            footprint,
            pin_to_pad_map: pin_pad_map,
            snapeda_source: SnapedaSource {
                unipart_id: unipart_id.to_string(),
                model_id: model_id.to_string(),
                ipc_package,
            },
        });
    }

    // No package found — return empty footprint
    let footprint = FootprintData {
        part_details: FootprintPartDetails {
            part_number: part_number.clone(),
            datasheet_revision: None,
        },
        packages: vec![],
    };

    Ok(SnapedaOutput {
        pinout,
        footprint,
        pin_to_pad_map: pin_pad_map,
        snapeda_source: SnapedaSource {
            unipart_id: unipart_id.to_string(),
            model_id: model_id.to_string(),
            ipc_package,
        },
    })
}

// --- Output helpers ---

fn serialize_json<T: Serialize>(value: &T, formatted: bool) -> Result<String, String> {
    if formatted {
        serde_json::to_string_pretty(value)
            .map_err(|e| format!("Failed to serialize output: {}", e))
    } else {
        serde_json::to_string(value)
            .map_err(|e| format!("Failed to serialize output: {}", e))
    }
}

fn write_output(text: &str, out: Option<PathBuf>) -> Result<(), String> {
    if let Some(path) = out {
        std::fs::write(&path, text)
            .map_err(|e| format!("Failed to write output file: {}", e))?;
        eprintln!("Wrote output to: {}", path.display());
    } else {
        println!("{}", text);
    }
    Ok(())
}

fn emit_output(
    json_output: bool,
    formatted: bool,
    out: Option<PathBuf>,
    output: &SnapedaOutput,
) -> Result<(), String> {
    let text = if json_output {
        serialize_json(output, formatted)?
    } else {
        format_full_human(output)
    };

    write_output(&text, out)
}

// --- Human-readable formatting ---

fn format_full_human(output: &SnapedaOutput) -> String {
    let mut s = String::new();

    let part_number = &output.pinout.part_details.part_number;

    let ipc = output
        .snapeda_source
        .ipc_package
        .as_deref()
        .unwrap_or("Unknown");

    let total_pads = output
        .footprint
        .packages
        .first()
        .map(|p| p.pin_count)
        .unwrap_or(0);

    s.push_str(&format!("SnapEDA: {}\n", part_number));
    s.push_str(&format!(
        "Package: {} ({} pads)\n",
        ipc, total_pads
    ));
    s.push_str(&format!(
        "Source:  unipart_id={}, model_id={}\n",
        output.snapeda_source.unipart_id, output.snapeda_source.model_id
    ));
    s.push('\n');

    // Pin map
    s.push_str("Pin Map:\n");
    s.push_str(&format!("  {:<4}  {:<14}  {}\n", "Pad", "Name", "Type"));
    s.push_str(&format!("  {:<4}  {:<14}  {}\n", "---", "----", "----"));

    if let Some(pkg) = output.pinout.packages.first() {
        // Collect thermal pad name from footprint for annotation
        let thermal_pad_nums: std::collections::HashSet<String> = output
            .footprint
            .packages
            .first()
            .and_then(|fp| fp.thermal_pad.as_ref())
            .map(|_| {
                // Determine which pad numbers are thermal from the pin map
                // A thermal pad is one not in fp_pads but in the connects
                let fp_pad_numbers: std::collections::HashSet<String> = output
                    .footprint
                    .packages
                    .first()
                    .map(|fp| fp.pads.iter().map(|p| p.number.to_string()).collect())
                    .unwrap_or_default();
                output
                    .pin_to_pad_map
                    .iter()
                    .map(|m| m.pad_number.clone())
                    .filter(|pn| !fp_pad_numbers.contains(pn))
                    .collect()
            })
            .unwrap_or_default();

        for pin in &pkg.pins {
            let thermal_note = if thermal_pad_nums.contains(&pin.pin_number) {
                "  (exposed pad)"
            } else {
                ""
            };
            s.push_str(&format!(
                "  {:>4}  {:<14}  {}{}\n",
                pin.pin_number, pin.pin_name, pin.electrical_type, thermal_note
            ));
        }
    }
    s.push('\n');

    // Footprint pads
    if let Some(fp_pkg) = output.footprint.packages.first() {
        s.push_str("Footprint Pads:\n");
        s.push_str(&format!(
            "  {:<4}  {:<16}  {:<16}  {}\n",
            "Pad", "Position", "Size", "Type"
        ));
        s.push_str(&format!(
            "  {:<4}  {:<16}  {:<16}  {}\n",
            "---", "--------", "----", "----"
        ));

        for pad in &fp_pkg.pads {
            let pos = format!("({}, {})", pad.x, pad.y);
            let size = format!("{} x {}", pad.width, pad.height);
            s.push_str(&format!(
                "  {:>4}  {:<16}  {:<16}  SMD\n",
                pad.number, pos, size
            ));
        }

        if let Some(ref tp) = fp_pkg.thermal_pad {
            let pos = format!("({}, {})", tp.x, tp.y);
            let size = format!("{} x {}", tp.width, tp.height);
            // Find the thermal pad number from pin_to_pad_map
            let fp_pad_numbers: std::collections::HashSet<u32> =
                fp_pkg.pads.iter().map(|p| p.number).collect();
            let thermal_pad_num = output
                .pin_to_pad_map
                .iter()
                .find(|m| m.pad_number.parse::<u32>().map(|n| !fp_pad_numbers.contains(&n)).unwrap_or(false))
                .map(|m| m.pad_number.clone())
                .unwrap_or_else(|| "EP".to_string());

            s.push_str(&format!(
                "  {:>4}  {:<16}  {:<16}  Thermal (no paste)\n",
                thermal_pad_num, pos, size
            ));
        }

        if let Some(ref cy) = fp_pkg.courtyard {
            s.push('\n');
            s.push_str(&format!(
                "Courtyard: {} x {}\n",
                cy.width, cy.height
            ));
        }
    }

    s
}

fn format_pinout_human(output: &SnapedaOutput) -> String {
    let mut s = String::new();
    let part_number = &output.pinout.part_details.part_number;
    s.push_str(&format!("Pinout: {}\n\n", part_number));

    if let Some(pkg) = output.pinout.packages.first() {
        s.push_str(&format!("Package: {}\n", pkg.package_name));
        s.push_str(&format!("  {:<4}  {:<14}  {}\n", "Pad", "Name", "Type"));
        s.push_str(&format!("  {:<4}  {:<14}  {}\n", "---", "----", "----"));
        for pin in &pkg.pins {
            s.push_str(&format!(
                "  {:>4}  {:<14}  {}\n",
                pin.pin_number, pin.pin_name, pin.electrical_type
            ));
        }
    }
    s
}

fn format_footprint_human(output: &SnapedaOutput) -> String {
    let mut s = String::new();
    let part_number = &output.footprint.part_details.part_number;
    s.push_str(&format!("Footprint: {}\n\n", part_number));

    if let Some(pkg) = output.footprint.packages.first() {
        s.push_str(&format!(
            "Package: {} / {} ({} pads)\n",
            pkg.package_code, pkg.package_name, pkg.pin_count
        ));
        s.push_str(&format!("Source: {}\n\n", pkg.pad_data_source));

        s.push_str(&format!(
            "  {:<4}  {:<16}  {:<16}  {}\n",
            "Pad", "Position", "Size", "Type"
        ));
        s.push_str(&format!(
            "  {:<4}  {:<16}  {:<16}  {}\n",
            "---", "--------", "----", "----"
        ));

        for pad in &pkg.pads {
            let pos = format!("({}, {})", pad.x, pad.y);
            let size = format!("{} x {}", pad.width, pad.height);
            s.push_str(&format!(
                "  {:>4}  {:<16}  {:<16}  SMD\n",
                pad.number, pos, size
            ));
        }

        if let Some(ref tp) = pkg.thermal_pad {
            let pos = format!("({}, {})", tp.x, tp.y);
            let size = format!("{} x {}", tp.width, tp.height);
            s.push_str(&format!(
                "  EP    {:<16}  {:<16}  Thermal (no paste)\n",
                pos, size
            ));
        }

        if let Some(ref cy) = pkg.courtyard {
            s.push('\n');
            s.push_str(&format!(
                "Courtyard: {} x {}\n",
                cy.width, cy.height
            ));
        }
    }
    s
}
