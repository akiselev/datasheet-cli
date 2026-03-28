# Research: Parametric Search Capabilities Across Electronics Distributor APIs

**Date:** 2026-03-17
**Goal:** Evaluate parametric filtering capabilities across Mouser, DigiKey, JLCPCB/LCSC, and Octopart/Nexar APIs to determine how to add parametric search to datasheet-cli.

## Current State in datasheet-cli

All three existing integrations (Mouser, DigiKey, JLCPCB) use keyword-only search:

- **Mouser** (`mouser.rs`): Uses `POST /api/v1/search/keyword` with a `KeywordSearchRequest` containing a keyword string, record count, and starting record. The only filtering option is `SearchOptions` which accepts `None | Rohs | InStock | RohsAndInStock`. No parametric filtering.
- **DigiKey** (`digikey.rs`): Uses `POST /products/v4/search/keyword` with a `KeywordSearchRequest` containing only `Keywords`, `RecordCount`, and `RecordStartPosition`. None of the available filter fields are used. No parametric filtering.
- **JLCPCB** (`jlcpcb.rs`): Uses `POST selectSmtComponentList/v2` with `keyword`, `currentPage`, `pageSize`, and an empty `componentAttributes: []`. The request body has fields for `firstSortName`, `secondSortName`, `componentLibraryType`, `componentBrand`, and `componentSpecification` but none are used.

All three APIs already support significantly more filtering than we currently use.

---

## API-by-API Analysis

### 1. DigiKey API v4 (Product Information)

**Endpoint:** `POST /products/v4/search/keyword`

**Authentication:** OAuth 2.0 client credentials (2-legged). Requires `X-DIGIKEY-Client-Id` header and Bearer token.

**Rate Limits:** 120 requests/minute, 1,000 requests/day for Product Information. Returns `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-BurstLimit-*` headers. 429 status when exceeded.

#### Parametric Filtering Capabilities

DigiKey v4 has the most complete parametric filtering of the distributor APIs. The `KeywordSearch` request body accepts a `FilterOptionsRequest` object with these filter types:

```json
{
  "Keywords": "LDO voltage regulator",
  "Limit": 50,
  "Offset": 0,
  "FilterOptionsRequest": {
    "ManufacturerFilter": [{ "Id": "123" }],
    "CategoryFilter": [{ "Id": "456" }],
    "StatusFilter": [{ "Id": "0" }],
    "PackagingFilter": [{ "Id": "1" }],
    "SeriesFilter": [{ "Id": "789" }],
    "MarketPlaceFilter": "ExcludeMarketPlace",
    "MinimumQuantityAvailable": 100,
    "ParameterFilterRequest": {
      "CategoryFilter": { "Id": "456" },
      "ParameterFilters": [
        {
          "ParameterId": 2079,
          "FilterValues": [{ "Id": "3.3V" }]
        }
      ]
    },
    "SearchOptions": ["InStock", "RohsCompliant", "HasDatasheet"]
  },
  "SortOptions": {
    "Field": "Price",
    "SortOrder": "Ascending"
  }
}
```

**Available Filters:**
| Filter | Description |
|--------|-------------|
| `ManufacturerFilter` | Array of manufacturer IDs |
| `CategoryFilter` | Array of DigiKey taxonomy/category IDs |
| `StatusFilter` | Active, obsolete, etc. |
| `PackagingFilter` | Tape & reel, cut tape, tube, etc. |
| `SeriesFilter` | Product series (e.g., "LM317") |
| `MarketPlaceFilter` | `NoFilter`, `ExcludeMarketPlace`, `MarketPlaceOnly` |
| `TariffFilter` | `None`, `ExcludeTariff`, `TariffOnly` |
| `MinimumQuantityAvailable` | Minimum stock level |
| `ParameterFilterRequest` | Parametric specs (voltage, current, package, etc.) |
| `SearchOptions` | Array: `InStock`, `HasDatasheet`, `Has3DModel`, `HasCadModel`, `RohsCompliant`, `NormallyStocking`, `NewProduct`, etc. |

**Two-Step Parametric Search Workflow:**

DigiKey's parametric filtering uses a faceted search pattern:

1. **Initial broad search** with keywords and optionally a category. The response includes a `FilterOptions` object listing all available parametric filters with their IDs, names, possible values, and result counts per value.
2. **Refined search** using the `ParameterId` and `FilterValues` from the first response's `FilterOptions.ParametricFilters`.

The `FilterOptions` response structure:
```json
{
  "FilterOptions": {
    "Manufacturers": [{ "Id": 1, "Value": "Texas Instruments", "ProductCount": 542 }],
    "Status": [{ "Id": 0, "Value": "Active", "ProductCount": 1200 }],
    "ParametricFilters": [
      {
        "ParameterId": 2079,
        "ParameterName": "Output Voltage",
        "FilterValues": [
          { "ValueId": "3.3V", "ValueName": "3.3V", "ProductCount": 89 },
          { "ValueId": "5V", "ValueName": "5V", "ProductCount": 67 }
        ]
      }
    ],
    "TopCategories": [
      {
        "RootCategory": { "Id": 32, "Name": "Integrated Circuits (ICs)" },
        "Category": { "Id": 685, "Name": "PMIC - Voltage Regulators - Linear" }
      }
    ]
  }
}
```

**Category Browsing:** `GET /products/v4/search/categories` returns the full taxonomy tree. `CategoryId` values can be passed to `FilterOptionsRequest.CategoryFilter` to restrict searches.

**Sorting:** Supports sorting by `Price`, `QuantityAvailable`, `Manufacturer`, `DigiKeyProductNumber`, `ManufacturerProductNumber`, `MinimumQuantity`, `Packaging`, `ProductStatus`, in `Ascending` or `Descending` order.

**Gaps:**
- No range filtering on parametric values (e.g., "voltage between 3.0V and 3.6V"). Filter values are discrete predefined options, not arbitrary ranges.
- Parameter IDs are opaque integers that must be discovered from a prior search response.
- Max 50 results per page.

---

### 2. Mouser API v1/v2

**Endpoints:**
| Endpoint | Version | Method |
|----------|---------|--------|
| `/api/v1/search/keyword` | v1 | POST |
| `/api/v1/search/partnumber` | v1 | POST |
| `/api/v2/search/keywordandmanufacturer` | v2 | POST |
| `/api/v2/search/partnumberandmanufacturer` | v2 | POST |

**Authentication:** API key passed as query parameter `?apiKey=...`

**Rate Limits:** 30 requests/minute, 1,000 requests/day.

#### Parametric Filtering Capabilities

Mouser's API has very limited filtering compared to DigiKey:

**v1 Keyword Search:**
```json
{
  "SearchByKeywordRequest": {
    "keyword": "LDO 3.3V",
    "records": 50,
    "startingRecord": 0,
    "searchOptions": "InStock",
    "searchWithYourSignUpLanguage": "en"
  }
}
```

`searchOptions` values: `None` (1), `Rohs` (2), `InStock` (4), `RohsAndInStock` (8). Only one at a time.

**v2 Keyword + Manufacturer Search:**
```json
{
  "SearchByKeywordMfrNameRequest": {
    "manufacturerName": "Texas Instruments",
    "keyword": "LDO 3.3V",
    "records": 50,
    "pageNumber": 1,
    "searchOptions": "InStock",
    "searchWithYourSignUpLanguage": "en"
  }
}
```

The v2 endpoint adds manufacturer name filtering and pagination via `pageNumber` (instead of `startingRecord`).

**What the response includes:**
Parts returned include a `ProductAttributes` field (JSON array) and a `Category` string, but these are output-only -- they cannot be used as input filters.

**Gaps:**
- **No parametric filtering whatsoever.** Cannot filter by voltage, current, package, temperature, or any electrical specification.
- **No category browsing API.** Cannot list categories or filter by category.
- The only filters are: keyword text, manufacturer name (v2 only), and `searchOptions` (RoHS/in-stock).
- Max 50 results per request.
- The Mouser website has parametric search, but it is not exposed through the API.

**Verdict:** Mouser API is the weakest for parametric search. Useful only for keyword and part number lookups. An LLM agent must craft very specific keyword queries (e.g., "LDO 3.3V SOT-223 1A") and rely on result descriptions for filtering.

---

### 3. JLCPCB/LCSC API

**Endpoint:** `POST https://jlcpcb.com/api/overseas-pcb-order/v1/shoppingCart/smtGood/selectSmtComponentList/v2`

**Authentication:** None required.

**Rate Limits:** Undocumented but appears to not be rate-limited aggressively (community projects scrape it successfully).

#### Parametric Filtering Capabilities

The JLCPCB search API has moderate parametric filtering through several fields:

```json
{
  "currentPage": 1,
  "pageSize": 100,
  "keyword": "LDO",
  "searchSource": "search",
  "componentLibraryType": "base",
  "preferredComponentFlag": true,
  "stockFlag": null,
  "stockSort": null,
  "firstSortName": "Voltage Regulators/Stabilizers",
  "secondSortName": "Low Dropout Regulators(LDO)",
  "componentBrand": "TEXAS INSTRUMENTS",
  "componentSpecification": "SOT-223",
  "componentAttributes": [
    {
      "attributeName": "Output Voltage",
      "attributeValue": "3.3V"
    }
  ]
}
```

**Available Filters:**
| Field | Description |
|-------|-------------|
| `keyword` | Free-text search |
| `componentLibraryType` | `"base"` (basic parts, cheaper assembly fee) or `"expand"` (extended parts) |
| `preferredComponentFlag` | `true` to show only JLCPCB preferred parts |
| `stockFlag` | Filter by in-stock status |
| `firstSortName` | Top-level category name (e.g., "Voltage Regulators/Stabilizers") |
| `secondSortName` | Sub-category name (e.g., "Low Dropout Regulators(LDO)") |
| `componentBrand` | Manufacturer/brand name string |
| `componentSpecification` | Package/footprint string (e.g., "SOT-223", "0402") |
| `componentAttributes` | Array of `{attributeName, attributeValue}` pairs for parametric filtering |

**Basic vs Extended Parts Filtering:**

This is a key feature for JLCPCB assembly. Setting `componentLibraryType: "base"` returns only basic parts (no extra fee for assembly). Setting it to `"expand"` returns extended parts. The `preferredComponentFlag` further narrows to JLCPCB's recommended parts within the extended category.

**Category Browsing:**

Category names (`firstSortName`, `secondSortName`) are strings, not IDs. The available category values must be discovered either by:
- Inspecting the JLCPCB parts page UI
- Using the jlcsearch third-party API (see below)
- Trial and error with known category names

**Component Attributes:**

The `componentAttributes` array allows parametric filtering, but the available attribute names and values are category-dependent and not documented. They mirror the attributes shown on the JLCPCB parts page for a given category. Common attributes include:
- Resistance, Capacitance, Inductance
- Voltage Rating, Current Rating
- Tolerance, Power Rating
- Operating Temperature

**Detail Endpoint:** `GET https://cart.jlcpcb.com/shoppingCart/smtGood/getComponentDetail?componentCode=C14663` returns full part details including all attributes.

**Gaps:**
- No documented API schema; the API is reverse-engineered from the JLCPCB website.
- Category names and attribute names are strings that must be known in advance.
- No range filtering on attribute values.
- API stability is not guaranteed (it is an internal API, not a public developer API).
- Max 100 results per page.

#### Alternative: jlcsearch API (tscircuit)

A third-party open-source API at `https://jlcsearch.tscircuit.com` provides better parametric search for JLCPCB parts:

- `GET /resistors/list.json?resistance=10k&package=0402` -- Resistors with specific resistance and package
- `GET /capacitors/list.json?package=0402` -- Capacitors by package
- `GET /voltage_regulators/list.json` -- Voltage regulators
- `GET /microcontrollers/list.json` -- Microcontrollers
- `GET /categories/list.json` -- All categories and subcategories
- `GET /components/list.json?subcategory_name=...&package=...` -- Generic search with subcategory
- `GET /api/search?q=...&package=...&limit=100` -- Full-text search

Each component-type endpoint has type-specific query parameters. Append `.json` to any listing page URL for JSON output.

**Advantages:** Purpose-built for parametric search, no auth required, component-type-specific filters.
**Disadvantages:** Third-party service (may go down), database may lag behind JLCPCB, limited documentation on all available filter parameters.

---

### 4. Octopart/Nexar API (GraphQL)

**Endpoint:** `POST https://api.nexar.com/graphql`

**Authentication:** OAuth 2.0 client credentials. Register at nexar.com for client ID and secret.

**Rate Limits / Pricing:**
| Plan | Monthly Part Limit | Notes |
|------|-------------------|-------|
| Evaluation (free) | 100 parts **lifetime** (not monthly) | No reset. Includes all features for testing. |
| Standard | 2,000 parts/month | Excludes lead time, lifecycle, datasheets, tech specs |
| Pro | 15,000 parts/month | Lead time and lifecycle as add-ons; no ECAD/similar parts |
| Enterprise | Custom | All features included |

**Important:** Limits are on **parts returned**, not queries. A single query returning 10 results costs 10 parts against your limit. Category queries cost nothing.

#### Parametric Filtering Capabilities

Nexar/Octopart has the most powerful parametric search of all options. Built on GraphQL, it supports true parametric filtering with range queries.

**Basic Parametric Search Query:**
```graphql
query {
  supSearch(
    q: "LDO voltage regulator"
    filters: {
      manufacturer_id: ["Texas Instruments"]
      category_id: ["4162"]
    }
    sort: "median_price_1000"
    sortDir: asc
    limit: 10
    start: 0
  ) {
    hits
    results {
      part {
        mpn
        manufacturer { name }
        shortDescription
        category { id name path }
        specs {
          attribute { name shortname }
          displayValue
        }
        bestDatasheet { url }
        medianPrice1000 { price currency }
        totalAvail
        sellers(authorizedOnly: true) {
          company { name }
          offers {
            inventoryLevel
            prices { quantity price currency }
          }
        }
      }
    }
  }
}
```

**Advanced Filtering with Ranges:**
```graphql
query {
  supSearch(
    q: "capacitor"
    filters: {
      manufacturer_id: ["Murata", "Samsung"]
      capacitance: ["(100n__1u)"]
      voltagerating_dc_: ["(16__)"]
      case_package: ["0402", "0603"]
    }
    sort: "median_price_1000"
    sortDir: asc
  ) {
    hits
    results {
      part {
        mpn
        manufacturer { name }
        specs {
          attribute { shortname }
          displayValue
        }
      }
    }
  }
}
```

**Filter Syntax:**
| Syntax | Meaning | Example |
|--------|---------|---------|
| `["value"]` | Exact match | `case_package: ["0402"]` |
| `["val1", "val2"]` | Match any (OR) | `manufacturer_id: ["TI", "Analog Devices"]` |
| `["(min__max)"]` | Range inclusive | `capacitance: ["(100n__1u)"]` |
| `["(__max)"]` | Less than or equal | `numberofpins: ["(__8)"]` |
| `["(min__)"]` | Greater than or equal | `voltagerating_dc_: ["(16__)"]` |

**Available Filter Keys:**
- `manufacturer_id` -- Manufacturer name or Nexar ID
- `category_id` -- Octopart category ID
- `distributor_id` -- Specific distributor
- `cad_models` -- `["symbol_footprint_3d"]` or `["symbol_footprint"]`
- Any Octopart spec attribute shortname (e.g., `resistance`, `capacitance`, `voltagerating_dc_`, `case_package`, `numberofpins`, `maxoutputvoltage`, `power`, `tolerance`, `operatingtemperature`)

**Sorting Options:**
- `median_price_1000` -- Median price at qty 1000
- `avg_price` -- Average price
- `num_authsuppliers` -- Number of authorized suppliers
- `mpn` -- Manufacturer part number
- `manufacturer.displayname` -- Manufacturer name
- Any spec shortname (e.g., `numberofpins`, `maxoutputvoltage`)

**Category Browsing:**
```graphql
query {
  supCategories {
    id
    name
    parentId
    numParts
    path
  }
}
```

**MPN Search with Specs:**
```graphql
query {
  supSearchMpn(q: "LM1117-3.3", limit: 5) {
    results {
      part {
        mpn
        manufacturer { name }
        category { name path }
        specs {
          attribute { name shortname }
          displayValue
        }
        bestDatasheet { url }
      }
    }
  }
}
```

**Multi-Part Match (BOM):**
```graphql
query {
  supMultiMatch(
    queries: [
      { mpn: "LM1117-3.3", manufacturer: "Texas Instruments" }
      { mpn: "STM32F103C8T6", manufacturer: "STMicroelectronics" }
    ]
  ) {
    hits
    results {
      part {
        mpn
        totalAvail
        medianPrice1000 { price currency }
      }
    }
  }
}
```

**Gaps:**
- Free tier is only 100 parts **lifetime**, making it useless for real work without a paid plan.
- Standard plan ($unknown/month) excludes lead time, lifecycle, and tech specs from responses.
- Mixing ranges with individual values in filters is unsupported and produces unpredictable results.
- Not all parts have all attributes; filtering on a spec attribute excludes parts that lack that attribute.
- Filter attribute shortnames must be known in advance (discoverable via schema introspection or Octopart docs).

---

## Comparison Matrix

| Capability | DigiKey v4 | Mouser v1/v2 | JLCPCB | Nexar/Octopart |
|------------|-----------|--------------|--------|----------------|
| **Keyword search** | Yes | Yes | Yes | Yes |
| **Category filtering** | Yes (by ID) | No | Yes (by name) | Yes (by ID) |
| **Category browsing** | Yes (endpoint) | No | No (undocumented) | Yes (GraphQL query) |
| **Manufacturer filter** | Yes (by ID) | Yes (v2, by name) | Yes (by name) | Yes (by name or ID) |
| **Package filter** | Yes (parametric) | No | Yes (componentSpecification) | Yes (case_package) |
| **Parametric specs** | Yes (faceted, discrete values) | No | Yes (componentAttributes) | Yes (range + exact) |
| **Range queries** | No | No | No | Yes |
| **Stock filter** | Yes (MinimumQuantityAvailable) | Yes (InStock option) | Yes (stockFlag) | Yes (requireStockAvailable) |
| **RoHS filter** | Yes (SearchOptions) | Yes (SearchOptions) | No | No (but RoHS in specs) |
| **Price sorting** | Yes | No | No | Yes (median_price_1000) |
| **Assembly cost filter** | N/A | N/A | Yes (basic/extended) | N/A |
| **Multi-part match** | No | No | No | Yes (supMultiMatch) |
| **Similar parts** | No | No | No | Yes (Enterprise only) |
| **Auth required** | Yes (OAuth) | Yes (API key) | No | Yes (OAuth) |
| **Rate limits** | 120/min, 1000/day | 30/min, 1000/day | Undocumented | By part count (see plans) |
| **Free tier** | Yes (with account) | Yes (with account) | Yes (no account) | 100 parts lifetime only |
| **Max results/page** | 50 | 50 | 100 | Configurable |

---

## Recommended Approach for datasheet-cli

### Architecture: Per-Distributor Parametric + Unified Interface

A fully unified parametric search across all distributors is impractical because:
1. Each API has different filter capabilities (Mouser has almost none, Nexar has ranges).
2. Filter value IDs are API-specific (DigiKey uses numeric IDs, JLCPCB uses strings).
3. The two-step discovery pattern (search, get filters, search again) works differently per API.

Instead, the recommended approach is:

#### 1. Common CLI Interface with Per-Distributor Flags

```bash
# Category-constrained search
datasheet digikey search "voltage regulator" --category "PMIC - Voltage Regulators - Linear"

# Parametric filtering (DigiKey)
datasheet digikey search "LDO" --in-stock --manufacturer "Texas Instruments" \
    --param "Output Voltage=3.3V" --param "Package / Case=SOT-223"

# JLCPCB with assembly category
datasheet jlcpcb search "LDO" --basic-only --category "Voltage Regulators/Stabilizers" \
    --subcategory "Low Dropout Regulators(LDO)" --package "SOT-223"

# Mouser (limited to keyword + manufacturer + stock)
datasheet mouser search "LDO 3.3V SOT-223" --manufacturer "Texas Instruments" --in-stock
```

#### 2. DigiKey: Two-Step Discovery Flow

Implement the faceted search pattern:

```bash
# Step 1: Broad search to discover available filters
datasheet digikey search "LDO" --show-filters --json

# Returns FilterOptions with ParametricFilters, categories, manufacturers
# The agent can inspect these and build a refined query

# Step 2: Apply discovered filters
datasheet digikey search "LDO" \
    --category-id 685 \
    --manufacturer-id 296 \
    --param-id "2079=3.3V" \
    --in-stock --json
```

Alternatively, implement a `--discover-filters` mode that returns just the filter options as structured JSON, which an LLM agent can use to construct the refined query.

#### 3. JLCPCB: Direct Attribute Filtering

Map the existing `componentAttributes` field:

```bash
datasheet jlcpcb search "capacitor" \
    --first-category "Capacitors" \
    --second-category "Multilayer Ceramic Capacitors(MLCC)" \
    --attr "Capacitance=100nF" \
    --attr "Voltage Rated=16V" \
    --package "0402" \
    --basic-only --json
```

#### 4. Nexar/Octopart: New Integration

Add Nexar as a fourth distributor source with the richest parametric search:

```bash
# Parametric search with ranges
datasheet nexar search "capacitor" \
    --category "Capacitors" \
    --manufacturer "Murata" \
    --spec "capacitance=(100n__1u)" \
    --spec "voltagerating_dc_=(16__)" \
    --spec "case_package=0402" \
    --sort "median_price_1000" --json

# Cross-reference: find a part across distributors
datasheet nexar part "LM1117-3.3" --json
```

**Implementation notes for Nexar:**
- OAuth 2.0 client credentials flow (similar to DigiKey)
- Environment variables: `NEXAR_CLIENT_ID`, `NEXAR_CLIENT_SECRET`
- GraphQL client needed (can use `ureq` + manual query strings, or a GraphQL client crate)
- Category discovery via `supCategories` query
- Spec attribute shortnames discoverable via schema introspection

#### 5. Priority Order for Implementation

1. **JLCPCB parametric** (low-hanging fruit) -- Already have the API integrated, just need to expose the existing `componentAttributes`, `firstSortName`, `secondSortName`, `componentLibraryType`, `componentBrand`, and `componentSpecification` fields as CLI flags. No new API calls needed.

2. **DigiKey parametric** (medium effort) -- Already have auth and search. Need to: (a) deserialize the `FilterOptions` from the response, (b) add a `--show-filters` mode, (c) add filter input flags, (d) construct the `FilterOptionsRequest` in the request body.

3. **Mouser v2 manufacturer filter** (low effort, limited value) -- Add the v2 `keywordandmanufacturer` endpoint with a `--manufacturer` flag. This is the only meaningful filter Mouser supports.

4. **Nexar/Octopart integration** (high effort, high value) -- New integration from scratch. Most powerful parametric search but requires OAuth setup, GraphQL client, and a paid plan for serious use. The free tier's 100-part lifetime limit is too small for agent-driven part selection.

---

## Example: LLM Agent Workflow for Finding a 3.3V LDO

With the proposed parametric search, an agent selecting a 3.3V LDO for a battery-powered project would:

```
1. Discover categories:
   $ datasheet digikey search "LDO voltage regulator" --show-filters --json
   -> Gets category ID for "PMIC - Voltage Regulators - Linear" (685)
   -> Gets parametric filter IDs for Output Voltage, Current Output, Package

2. Parametric search on DigiKey:
   $ datasheet digikey search "LDO" --category-id 685 \
       --param "Output Voltage=3.3V" \
       --param "Current - Output=1A" \
       --in-stock --sort price --json --limit 20

3. Cross-reference on JLCPCB for assembly availability:
   $ datasheet jlcpcb search "LDO 3.3V 1A" \
       --first-category "Voltage Regulators/Stabilizers" \
       --second-category "Low Dropout Regulators(LDO)" \
       --basic-only --json

4. Get details for candidates:
   $ datasheet digikey part "AP2112K-3.3TRG1" --json
   $ datasheet jlcpcb part "C51118" --json

5. Download datasheets for final candidates:
   $ datasheet digikey download "AP2112K-3.3TRG1" --dir ./datasheets
```

Without parametric search, the agent must:
- Craft very specific keyword strings ("LDO 3.3V 1A SOT-23-5 in stock")
- Hope the keyword search returns relevant results
- Post-filter results by parsing descriptions (brittle)
- Make many more API calls iterating on keyword variations

---

## Open Questions

1. **Nexar pricing:** Standard and Pro plan costs are not publicly listed. Need to check if the free evaluation tier (100 parts lifetime) is sufficient for initial development and testing.

2. **JLCPCB attribute discovery:** The available `componentAttributes` names are not documented. Should we add a command to list available attributes for a given category? This could be done by fetching a few parts from a category and inspecting their attributes.

3. **DigiKey filter ID caching:** The two-step discovery flow requires an extra API call. Should we cache known filter IDs locally (category -> parameter IDs mapping) to avoid the discovery step for common searches?

4. **jlcsearch as alternative JLCPCB backend:** The tscircuit jlcsearch API at `jlcsearch.tscircuit.com` provides cleaner parametric search but is a third-party service. Worth supporting as a fallback or alternative?

5. **Unified output format:** All parametric search results should normalize to a common output schema regardless of source, including: MPN, manufacturer, description, key specs, price, stock, datasheet URL, and distributor-specific metadata (JLCPCB assembly category, DigiKey part status, etc.).

## Sources

- [Mouser Search API](https://www.mouser.com/api-search/)
- [Mouser API Documentation](https://api.mouser.com/api/docs/ui/index)
- [DigiKey API v4 KeywordSearch](https://developer.digikey.com/products/product-information-v4/productsearch/keywordsearch)
- [DigiKey API v4 Categories](https://developer.digikey.com/products/product-information-v4/productsearch/categories)
- [DigiKey API Documentation](https://developer.digikey.com/documentation)
- [DigiKey API v4 Blog Post](https://briankhuu.com/blog/2024/09/17/playing-around-with-digikey-api/)
- [JLCPCB Parts Library](https://jlcpcb.com/parts/componentSearch)
- [LCSC API Documentation (yaqwsx/jlcparts)](https://github.com/yaqwsx/jlcparts/blob/master/LCSC-API.md)
- [JLCPCB Parts Database Scraper](https://github.com/CDFER/jlcpcb-parts-database)
- [jlcsearch API Documentation (tscircuit)](https://docs.tscircuit.com/web-apis/jlcsearch-api)
- [Nexar API](https://nexar.com/api)
- [Nexar API Plans Comparison](https://nexar.com/compare-plans)
- [Nexar GraphQL Query Examples](https://support.nexar.com/support/solutions/articles/101000494582-nexar-playground-graphql-query-examples)
- [Nexar Sorting and Filtering](https://support.nexar.com/support/solutions/articles/101000452264-supply-sorting-and-filtering-your-queries)
- [Nexar Part Limits](https://support.nexar.com/support/solutions/articles/101000476314-part-limits-and-how-they-work)
- [Nexar Query Templates](https://support.nexar.com/support/solutions/articles/101000472564-query-templates)
- [sparkmicro/mouser-api (Python)](https://github.com/sparkmicro/mouser-api)
- [digikey-apiv4 (Python)](https://pypi.org/project/digikey-apiv4/)
