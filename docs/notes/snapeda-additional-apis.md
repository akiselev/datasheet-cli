# SnapEDA / SnapMagic Additional APIs Research

Date: 2026-03-17

## Background

datasheet-cli already integrates with SnapEDA (now branded SnapMagic Search) for:
- Part search via `POST /api/v1/search_local_internal`
- Unipart info via `GET /api/get_part_for_unipart/{unipart_id}`
- Eagle XML (symbol+footprint+deviceset) via `GET /api/get_html_5/{model_id}` (also aliased as `get_file_for_html5_web`)
- Footprint IPC name via `GET /parts/snapdata/footprint-info/{unipart_id}/{model_id}/json`
- Authenticated login via `POST /account/login/`
- Authenticated download via `GET /parts/snapapi/download-component/{model_id}/{unipart_id}/{format}`

This note documents all additional API endpoints discovered through network traffic analysis on the SnapEDA website, web search, and browser exploration.

---

## Discovered API Endpoints (Not Yet Integrated)

### 1. Part Alternatives / Cross-References

**Endpoint:** `GET /api/get_alternatives/?unipart_id={unipart_id}&count={N}`

**Response (JSON):**
```json
{
  "alternatives_ab_version": "a",
  "alternatives": [
    {
      "has_symbol": 1,
      "has_footprint": 1,
      "has_3d": 0,
      "unipart_id": 10566918,
      "manufacturer": "Octavo Systems",
      "part_name": "OSD3358512MBCB",
      "description": "OSD335x Embedded Module...",
      "package": "BGA-302",
      "part_url": "/parts/OSD3358512MBCB/Octavo%20Systems/view-part/",
      "part_image": "https://snapeda.s3.amazonaws.com/partpics/...",
      "manuf_img": "pinax/images/company_logos/thumbs/..."
    }
  ]
}
```

**Value for datasheet-cli:** Could provide automated part alternatives/cross-references during part selection (Phase 1). Note: the alternatives shown are often sponsored placements (ads) rather than true functional equivalents. The data leans heavily toward paid partner parts (e.g., Octavo Systems appears frequently regardless of the query). Use with caution -- these are not true parametric cross-references.

---

### 2. Distributor Pricing & Availability

SnapEDA proxies pricing data from multiple distributors. These return rich JSON with stock levels, price breaks, compliance data, and buy links.

**DigiKey:**
`GET /api/getDigiKeyInfoAPI/{part_name}/{unipart_id}/{country_code}`

Response includes: `quantity`, `quantity_on_order`, `price`, `DK_part_number`, `buyUrl`, `currency`, `prices` (array of price breaks with `TotalPrice`, `UnitPrice`, `BreakQuantity`), `datasheet` URL, `image_url`, `description`.

**Mouser:**
`GET /api/get_mouser_info_api?part_name={part}&manufacturer={mfr}`

Response includes: `Category`, `ROHSStatus`, `Availability` (stock count), `PriceBreaks` (with currency and quantity tiers), `ProductDetailUrl`, `DataSheetUrl`, `LeadTime`, `LifecycleStatus`, `ProductCompliance` (USHTS, CNHTS, JPHTS, TARIC, ECCN codes), `SuggestedReplacement`, `ProductAttributes`, `AlternatePackagings`, `SurchargeMessages` (tariff info).

**Winsource:**
`GET /api/get_winsource_electronics_api?part_name={part}&manufacturer={mfr}`

Response includes: `quantity`, `quantity_on_order`, `price`, `buy_url`.

**RS Components:**
`GET /api/get_rs_components_info_api?part_name={part}&manufacturer={mfr}`

Response: typically `{}` when no data is available.

**Value for datasheet-cli:** The Mouser endpoint is particularly rich -- it returns compliance data (HTS codes, ECCN), RoHS status, lifecycle status, suggested replacements, and tariff/surcharge info that we don't get from our direct Mouser API integration. The DigiKey endpoint provides a convenient pre-aggregated view. These could supplement our existing distributor integrations or serve as a fallback when API keys are not configured.

**Important caveat:** These endpoints proxy SnapEDA's own distributor API keys. Using them at scale would be piggybacking on their rate limits. Best suited for spot-checks, not bulk queries.

---

### 3. Related/Complementary Parts

**Endpoint:** `GET /api/get_relevant_part_api/{unipart_id}/{category_path}/{index}/?ref={ref}`

Returns a single relevant part from the same category. The response includes `type` ("competing"), `relevancy` ("high"), `has_in_stock_offers`, and part details. These are largely sponsored/advertising placements (TE Connectivity ads appear frequently).

**Endpoint:** `POST /api/get_related_parts`
Body: `part_name=...&manufacturer=...&unipart_id=...`

Returns `{"results": [...]}` -- appears to return related parts that may be functionally connected (companion ICs, antennas for RF modules, etc.).

**Endpoint:** `GET /api/get_complementary_TE_part_api/{unipart_id}/?cat={category}`
Optional: `&force_3d=true` to filter for parts with 3D models.

TE Connectivity-specific complementary parts. These are ads -- TE pays SnapEDA to display these alongside competitor parts.

---

### 4. 3D Model Viewer & Download

**Viewer URL:**
`GET /parts/get-3d-viewer-url/{model_id}/?unipart_id={unipart_id}`

Returns a URL like:
```
https://3d.snapeda.com/?url=https://snapeda.s3.amazonaws.com/media/CADParts/{part}--3DModel-fbx-{id}.glb?AWSAccessKeyId=...&Expires=...&Signature=...&file_unit=&unipart_id=...&part_id=...
```

The 3D model is served as a **GLB file** (binary glTF) from S3 with time-limited signed URLs. The viewer at `3d.snapeda.com` uses Babylon.js v4.2.1 to render it.

**3D Info from Third Parties:**
`GET /api/get_3d_info_from_3rd_parties/?has_3d_models={0|1}&unipart={unipart_id}&step_model={model_id}&is_bot=false&formatted_part_number={part_name}`

Returns: `{"has_3d_models": "", "source_3d_models": "", "cad_availability": ""}`

This checks if 3D models are available from external sources (e.g., TraceParts, 3DContentCentral).

**STL Dimensions:**
`POST https://3d.snapeda.com/get-stl-dimensions`

Returns physical dimensions extracted from the 3D model.

**STEP Download:**
`GET /parts/snapapi/download-component/{model_id}/{unipart_id}/step`

Currently returns `{"error": "File is not in supported format"}` for most parts. SnapEDA's 3D models appear to be primarily in GLB/FBX format internally, with STEP being a format they export to on-demand. The `step` format may only work for parts where a native STEP file was uploaded by the manufacturer.

**Value for datasheet-cli:** The GLB file URL from the viewer endpoint can be extracted for direct download. This gives us access to 3D model geometry. For PCB design, we would need STEP format -- the download-component endpoint with `step` format may work for some parts but is unreliable. A potential workaround is downloading the GLB and converting with an external tool, though this loses manufacturing-precision geometry.

---

### 5. Download Count & Standards

**Download Count:**
`GET /api/get_num_downloads/{unipart_id}`

Returns a plain integer (e.g., `2455`). Useful as a popularity/trust signal during part selection.

**Standards Used:**
`GET /api/get_standard_used/{model_id}`

Returns a text string like: `"This part was created using the manufacturer's recommendations based on datasheet Revision 1.0."`

Indicates whether the part was created per IPC standards or manufacturer recommendations, and which datasheet revision was used.

---

### 6. Verification/Validation Results

**Endpoint:** `GET /parts/snapdata/validator-results/{unipart_id}/{model_id}/symbol/_/html`

Returns an HTML fragment containing the results of SnapEDA's patented "Verification Checker" -- automated checks for:
- **Manufacturability**: silkscreen clearance, solder mask/paste dimensions, component orientation
- **Schematic**: pin-related checks
- **Documentation**: reference designator, description presence
- **Miscellaneous**: other checks

Each category is graded Pass / Please Inspect / Fail.

**Value for datasheet-cli:** This could be exposed as a quality signal for parts. Before committing to using a SnapEDA symbol/footprint, we could check validation results to flag potential issues. Would need HTML parsing to extract structured pass/fail data.

---

### 7. Part Comments & Issues

**Endpoint:** `GET /parts/{unipart_id}/part-comments-unipart/`

Returns an HTML fragment showing user-reported issues for the part's CAD models. Could extract issue count and descriptions to flag known problems.

---

### 8. User Library Management

**Endpoint:** `GET /parts/snapapi/part-in-library/{model_id}/{unipart_id}/`

Returns `"Add to Library"` or similar text indicating whether the part is already in the user's personal library.

**Endpoint:** `GET /libraries/me/recent-downloads/`

Returns the user's recent download history (HTML page, not JSON).

**Endpoint:** `GET /instapart_api/get_user_credits/`

Returns: `{"user_credits": 0}` -- the user's InstaPart request credits.

---

### 9. Datasheet Access

**Endpoint:** `GET /parts/{part_name}/{manufacturer}/datasheet/`

Redirects to or serves the part's datasheet PDF. This is a web page, not a direct PDF link, but it provides access to the datasheet that SnapEDA has on file.

---

### 10. Stripe Integration (Premium Features)

**Endpoint:** `GET /premium/publishable-key/`

Returns a Stripe publishable key (`pk_live_...`), indicating premium/paid features are handled via Stripe billing. The premium tier likely unlocks higher download limits or API access.

---

### 11. SnapMagic Copilot API

The Copilot widget (embedded on every page at `copilot.snapmagic.com`) is an AI chatbot that can:
- Answer electronics design questions
- Generate schematic suggestions
- Recommend parts
- Trigger CAD model downloads

It communicates via:
- `GET https://copilot-lang-graph-ef0f4fa266b6.herokuapp.com/api/chat-suggestions?token=...`
- Internal LangGraph-based agent on Heroku

This is not a general-purpose API -- it requires the embedded widget context and a session token. Not practical for CLI integration.

---

## Download Formats Available

From the existing `download-component` endpoint, the following formats are supported:

| Format String | File Type | Description |
|---|---|---|
| `altium_native` | `.SchLib`, `.PcbLib`, `.IntLib` | Native Altium Designer libraries |
| `eagle` | `.lbr` | Eagle library (XML-based, easily parseable) |
| `kicad` | `.kicad_sym` | KiCad schematic symbol |
| `kicad_mod` | `.kicad_mod` | KiCad footprint |
| `kicad_modv6` | `.kicad_mod` | KiCad v6+ footprint format |
| `step` | `.step` | 3D STEP model (limited availability) |
| `orcad` | ? | OrCAD/Allegro format |
| `pads` | ? | Mentor PADS format |
| `diptrace` | ? | DipTrace format |
| `proteus` | ? | Proteus format |
| `pcb123` | ? | PCB123 format |
| `designspark` | ? | DesignSpark format |
| `pulsonix` | ? | Pulsonix format |
| `cr5000` | ? | CR-5000/CR-8000 format |
| `expresspcb` | ? | ExpressPCB Plus format |
| `target3001` | ? | TARGET 3001! format |
| `ecadstar` | ? | eCADSTAR format |

Formats beyond altium_native, eagle, kicad, and kicad_mod have not been tested. The download endpoint returns a JSON response with a `url` field pointing to a signed S3 URL for the actual file (usually a ZIP archive).

---

## 3D Model Availability

- 3D models are stored as **GLB (binary glTF)** files on S3
- The viewer uses **Babylon.js** to render them in-browser
- STEP download via the download-component API is unreliable -- many parts return "File is not in supported format"
- The GLB files contain geometry with reasonable fidelity for visualization but may not have the precision needed for mechanical CAD (collision checking, enclosure design)
- S3 URLs are time-limited (signed with `AWSAccessKeyId`, `Expires`, `Signature`)
- 3D model availability is indicated by `has_3d` field in search results

**Practical approach for STEP files:** Download the `altium_native` format, which includes `.IntLib` files that sometimes embed STEP models. Alternatively, use the GLB URL from the viewer endpoint for basic geometry.

---

## Competitor Comparison

### SamacSys / Component Search Engine (by Supplyframe)

- **API access:** Not publicly documented. Requires contacting sales. Powers the "Library Loader" desktop app and integrations with Mouser, DigiKey, RS Components
- **Coverage:** Claims millions of parts, similar scale to SnapEDA
- **Data model:** Provides symbols, footprints, 3D models. No documented REST API for programmatic access
- **Integration method:** Desktop app (Library Loader) that integrates with KiCad, Altium, Eagle, etc. No CLI tool available
- **Strengths:** Deep distributor integration (powers Mouser's component pages). Parts are created by in-house engineers per IPC/manufacturer standards
- **Weaknesses:** No public API, no CLI access, locked into their desktop app ecosystem
- **Verdict:** Not suitable for CLI integration without a partnership agreement

### Ultra Librarian

- **API access:** No public REST API documented. Uses proprietary `.bxl` file format with a free "Reader" tool for conversion
- **Coverage:** Claims 16+ million verified models. Partners directly with semiconductor manufacturers (TI, ADI, TE Connectivity)
- **Data model:** Vendor-neutral `.bxl` format, exported to 30+ CAD tool formats
- **Integration method:** Web download or integration via Octopart/SiliconExpert. No programmatic CLI access documented
- **Strengths:** Source-of-truth model -- works directly with manufacturers. Best quality for manufacturer-partnered parts. Supports more output formats than SnapEDA
- **Weaknesses:** Closed ecosystem, `.bxl` is a proprietary binary format, no public API, free tier is read-only
- **Verdict:** Not suitable for CLI integration. The `.bxl` format would require reverse-engineering

### Octopart (by Altium/Nexar)

- **API access:** GraphQL API via Nexar platform. Well-documented. Requires API key (free tier: 500 queries/month, paid tiers available)
- **Coverage:** Aggregates from multiple CAD libraries (including Ultra Librarian). Primarily a search/pricing aggregator, not a CAD library itself
- **Data model:** Part specs, pricing, availability, datasheets. Links to CAD model downloads (via Ultra Librarian)
- **Strengths:** Best-documented API among all competitors. Rich parametric search. Aggregates pricing from many distributors
- **Weaknesses:** Free tier is very limited (500/month). CAD models come from Ultra Librarian (indirect). Expensive at scale
- **Verdict:** Worth integrating for parametric search and pricing comparison. Not a direct competitor to SnapEDA for CAD model data

### TraceParts / 3DContentCentral

- **API access:** Limited/undocumented public APIs. TraceParts has some REST endpoints
- **Coverage:** 3D models primarily. Strong for mechanical/connector parts
- **Strengths:** STEP/IGES/SAT format 3D models with manufacturing precision
- **Weaknesses:** Not focused on EDA symbols/footprints. Registration required for downloads
- **Verdict:** Could supplement SnapEDA for 3D STEP models specifically

---

## Opportunities for Deeper Integration

### High Value (Recommended)

1. **Distributor pricing proxy** -- The Mouser and DigiKey endpoints return richer data than our direct API integration (compliance codes, lifecycle status, tariffs). Could use as supplementary data source when direct API keys are not configured. Low effort to implement.

2. **Verification checker results** -- Parse the validator-results HTML to extract pass/fail status per category. Would provide a quality confidence score before using a SnapEDA part. Medium effort (HTML parsing).

3. **Download count** -- Simple integer endpoint, useful as a trust/popularity signal. Trivial to implement.

4. **3D model GLB download** -- Extract the S3 URL from the viewer endpoint for direct GLB file download. Would enable basic 3D model access even when STEP is unavailable. Low-medium effort.

### Medium Value

5. **Standards info** -- Which datasheet revision was used, whether IPC standards were applied. Useful metadata for quality assessment. Trivial to implement.

6. **Alternatives** -- The alternatives endpoint returns cross-reference data, though it is heavily influenced by advertising. Could be useful with appropriate filtering/disclaimers. Low effort.

7. **Part issues/comments** -- Scrape known issues to warn users about problematic CAD models. Medium effort (HTML parsing).

### Lower Value

8. **Additional download formats** -- Test and support more formats (orcad, pads, etc.) via the existing download-component endpoint. Just needs format string mapping.

9. **Copilot integration** -- The AI assistant could theoretically be useful for part recommendations, but the API is session-bound and not designed for programmatic access.

---

## Rate Limiting & Terms of Service

### Observed Rate Limiting
- No explicit rate limiting headers observed on API responses
- The existing integration adds 1-second delays between requests as a courtesy
- The Eagle XML endpoint (`get_html_5`) can return empty responses intermittently (we already retry 3x)
- No evidence of IP-based blocking during testing, but SnapEDA uses Cloudflare which could enforce limits

### Terms of Service Concerns
- Section 5.1(f): Prohibits distributing parts of the Site within other software
- Section 5.1(g): Cannot distribute more than 10 Design Files through any one location without written permission
- Section 3.1: Cannot create tools that aggregate Design Files for a competing site
- Design Files are available under **CC BY-SA 4.0** for individual use, but bulk redistribution requires written permission
- The undocumented APIs are not covered by any public API agreement -- using them is at your own risk
- SnapEDA's official API (at /get-api/) requires contacting sales and is a separate commercial product

### Recommendations
- Continue using the current approach: download individual parts on-demand for specific projects
- Do not cache or redistribute downloaded CAD files beyond the project scope
- The pricing/distributor proxy endpoints should be used sparingly (they consume SnapEDA's own API quotas)
- For production/commercial use, consider contacting SnapEDA about their official API offering

---

## Summary of All Known SnapEDA API Endpoints

### Already Integrated in datasheet-cli
| Endpoint | Method | Description |
|---|---|---|
| `/api/v1/search_local_internal` | POST | Search for parts |
| `/api/get_part_for_unipart/{uid}` | GET | Get unipart info (model_id, name, manufacturer) |
| `/api/get_html_5/{model_id}` | GET | Get Eagle XML (symbol + footprint + deviceset) |
| `/api/get_file_for_html5_web/{model_id}` | GET | Alias for get_html_5 |
| `/parts/snapdata/footprint-info/{uid}/{mid}/json` | GET | Get IPC package name |
| `/account/login/` | POST | Authenticate (Django login) |
| `/parts/snapapi/download-component/{mid}/{uid}/{fmt}` | GET | Download CAD files (auth required) |

### Discovered But Not Yet Integrated
| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/api/get_alternatives/?unipart_id={uid}&count={N}` | GET | No | Part alternatives/cross-references |
| `/api/getDigiKeyInfoAPI/{part}/{uid}/{country}` | GET | No | DigiKey pricing & availability |
| `/api/get_mouser_info_api?part_name={p}&manufacturer={m}` | GET | No | Mouser pricing, compliance, lifecycle |
| `/api/get_winsource_electronics_api?part_name={p}&manufacturer={m}` | GET | No | Winsource pricing & stock |
| `/api/get_rs_components_info_api?part_name={p}&manufacturer={m}` | GET | No | RS Components pricing |
| `/api/get_num_downloads/{uid}` | GET | No | Download count (popularity) |
| `/api/get_standard_used/{model_id}` | GET | No | Standards/datasheet revision used |
| `/api/get_3d_info_from_3rd_parties/?...` | GET | No | 3D model availability from external sources |
| `/api/get_relevant_part_api/{uid}/{category}/{idx}/` | GET | No | Sponsored relevant parts |
| `/api/get_complementary_TE_part_api/{uid}/?cat={cat}` | GET | No | TE Connectivity complementary parts (ads) |
| `/api/get_related_parts` | POST | No | Related/companion parts |
| `/api/save_symbol_footprint_image/` | POST | Session | Save rendered preview images |
| `/parts/get-3d-viewer-url/{model_id}/?unipart_id={uid}` | GET | No | 3D viewer URL (contains S3 GLB link) |
| `/parts/{uid}/part-comments-unipart/` | GET | No | User issues/comments (HTML) |
| `/parts/{mid}/{uid}/viewmodelinfo2/` | GET | No | Model metadata, creator info (HTML) |
| `/parts/snapdata/validator-results/{uid}/{mid}/symbol/_/html` | GET | No | Verification checker results (HTML) |
| `/parts/snapapi/part-in-library/{mid}/{uid}/` | GET | Session | Check if part is in user's library |
| `/instapart_api/get_user_credits/` | GET | Session | User's InstaPart credits |
| `/premium/publishable-key/` | GET | No | Stripe publishable key for premium |
| `/libraries/me/recent-downloads/` | GET | Session | User's download history |
| `https://3d.snapeda.com/get-stl-dimensions` | POST | No | Physical dimensions from 3D model |
| `https://pricing.snapeda.com/search?q={query}` | GET | No | SnapPricing multi-distributor comparison |

### SnapPricing (Separate Service)
- Hosted at `pricing.snapeda.com`
- Aggregates pricing from: DigiKey, Mouser, Arrow, Toby Electronics, OnlineComponents
- Web-only interface, no documented API
- Shares authentication with main SnapEDA site
