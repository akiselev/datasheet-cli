# BOM Management & Multi-Distributor Optimization — Research Note

Date: 2026-03-17

## 1. Current Pricing/Stock Data Available from Integrated Distributors

### Mouser (`src/mouser.rs`)

The Mouser integration returns the following per-part data relevant to BOM management:

| Field | Type | Notes |
|-------|------|-------|
| `manufacturer_part_number` | String | MPN |
| `mouser_part_number` | String | Distributor SKU |
| `availability_in_stock` | String | Stock count (as string, e.g. "1500 In Stock") |
| `availability_on_order` | JSON Value | Can be string or array of objects |
| `lead_time` | String | e.g. "6 Weeks" |
| `lifecycle_status` | String | e.g. "New Product", "Not Recommended for New Designs" |
| `min` | String | Minimum order quantity |
| `mult` | String | Order multiple |
| `price_breaks` | Vec | Each: `{ quantity: i32, price: String, currency: String }` |
| `suggested_replacement` | String | Alternative part |
| `rohs_status` | String | Compliance info |
| `reeling` | bool | Whether cut tape/reel options exist |

**Key observations:**
- Price breaks are strings (e.g. "$0.0230") not floats — need parsing.
- Stock is a string, not an integer — needs parsing (may contain text like "In Stock").
- Lead time is free-text, not structured days.
- The Mouser search API does NOT support batch queries for multiple different MPNs in a single request. Each MPN requires a separate API call.
- API key is free to obtain with a Mouser account.

### DigiKey (`src/digikey.rs`)

| Field | Type | Notes |
|-------|------|-------|
| `manufacturer_part_number` | String | MPN |
| `digi_key_part_number` | String | Distributor SKU |
| `quantity_available` | i32 | Stock count (integer — better than Mouser) |
| `manufacturer_public_quantity` | i32 | Manufacturer's reported stock |
| `minimum_order_quantity` | i32 | MOQ |
| `unit_price` | f64 | Base unit price |
| `standard_pricing` | Vec | Each: `{ break_quantity: i32, unit_price: f64, total_price: f64 }` |
| `part_status` | String | Lifecycle status |
| `packaging` | PackagingInfo | e.g. "Cut Tape", "Tape & Reel" |
| `parameters` | Vec | Technical parameters (key-value pairs) |

**Key observations:**
- Pricing is already numeric (f64) — much cleaner than Mouser.
- Stock is integer, not string.
- DigiKey API v4 supports batch product details (up to 50 parts per request).
- Uses OAuth2 client_credentials flow. Requires `DIGIKEY_CLIENT_ID` + `DIGIKEY_CLIENT_SECRET`.
- No lead time field is currently deserialized (may be available in the API but not captured).

### JLCPCB/LCSC (`src/jlcpcb.rs`)

| Field | Type | Notes |
|-------|------|-------|
| `lcsc_part_number` | String | LCSC part number (Cxxxxx format) |
| `manufacturer_part_number` | String | MPN |
| `stock` | i64 | Stock count |
| `price_breaks` | Vec | Each: `{ quantity: i32, price_usd: f64 }` |
| `category` | String | "basic", "preferred", or "extended" (assembly category) |
| `assembly_process` | String | "SMT" or "THT" |
| `minimum_order` | i32 | MOQ |
| `package` | String | Footprint/package name |
| `first_category` / `second_category` | String | Component classification |
| `attributes` | Vec | Technical attributes (key-value pairs) |

**Key observations:**
- Pricing is already numeric (f64), in USD.
- Stock is integer.
- Assembly category (basic/preferred/extended) is critical for JLCPCB assembly cost — basic parts have no setup fee, extended parts cost extra.
- No API key required.
- No lead time field available.

### SnapEDA (`src/snapeda.rs`)

SnapEDA provides CAD models (symbols, footprints) but no pricing or stock data. Not relevant for BOM pricing, but useful for mapping MPNs to footprints.

### Summary of Gaps

| Feature | Mouser | DigiKey | JLCPCB |
|---------|--------|---------|--------|
| Numeric stock | No (string) | Yes | Yes |
| Numeric pricing | No (string) | Yes | Yes |
| Lead time | Yes (string) | No | No |
| MOQ | Yes (string) | Yes | Yes |
| Order multiple | Yes (string) | No | No |
| Batch query | No | Yes (50 parts) | No |
| Price currency | Per break | USD only | USD only |
| Lifecycle status | Yes | Yes | No |
| Assembly category | N/A | N/A | Yes |

## 2. BOM Tool API Landscape

### Nexar / Octopart API (recommended aggregator)

**What it is:** Nexar is Altium's data platform that powers Octopart. It provides a single GraphQL API that aggregates pricing and stock from 70+ million parts across all major distributors.

**API details:**
- Endpoint: `https://api.nexar.com/graphql/`
- Authentication: OAuth2 client credentials
- Query language: GraphQL
- Key query: `supMultiMatch` — accepts an array of MPN queries, returns parts with sellers, offers, pricing, stock, MOQ

**Pricing tiers:**
| Plan | Matched Parts/Month | Lead Time | Lifecycle | Datasheets | Tech Specs | Price |
|------|---------------------|-----------|-----------|------------|------------|-------|
| Evaluation | 100 | Yes | Yes | Yes | Yes | Free |
| Standard | 2,000 | No | No | No | No | Paid |
| Pro | 15,000 | Yes | Yes | Yes | Yes | Paid |
| Enterprise | Custom | Yes | Yes | Yes | Yes | Custom |

**How limits work:**
- A "matched part" is any part returned by a supply query.
- If you query 10 MPNs and get 3 results each, that's 30 matched parts.
- `supMultiMatch` returns 3 parts per query by default (configurable).
- Limits reset monthly on the 1st, no rollover.
- Design queries (prefixed `des`) are free.

**Example query for BOM pricing:**
```graphql
query BomPricing($queries: [SupPartMatchQuery!]!) {
  supMultiMatch(
    currency: "USD"
    queries: $queries
  ) {
    hits
    parts {
      mpn
      manufacturer { name }
      medianPrice1000 { quantity convertedPrice convertedCurrency }
      sellers(authorizedOnly: true) {
        company { name id }
        offers {
          sku
          inventoryLevel
          moq
          prices { quantity convertedPrice convertedCurrency }
          packaging
        }
      }
    }
  }
}
```

Variables:
```json
{
  "queries": [
    { "mpn": "ESP32-C6-WROOM-1U-N8", "limit": 3 },
    { "mpn": "AO3401A", "limit": 3 },
    { "mpn": "DW01A", "limit": 3 }
  ]
}
```

**Advantages over direct distributor APIs:**
- Single query for all distributors (no need to hit Mouser, DigiKey, etc. separately).
- Normalized data format across distributors.
- Includes distributors we don't have direct API access to (Arrow, Newark, Farnell, RS, TME, etc.).
- Lead time data (on Pro plan).
- `medianPrice1000` gives quick cost estimates without parsing price breaks.

**Disadvantages:**
- Free tier is only 100 parts/month — barely enough for one BOM.
- Standard tier (2,000 parts) loses lead time and lifecycle data.
- Pro tier pricing not publicly listed.
- Adds an external dependency and potential single point of failure.
- Cannot place orders through it (need to go to each distributor).

### FindChips (Supplyframe)

FindChips aggregates pricing and stock data from multiple distributors, similar to Octopart. It was acquired by Supplyframe (which was later acquired by Siemens). It offers:
- Real-time pricing and inventory from leading distributors
- Parametric search with attribute filtering
- Price comparison across distributors
- Alerts for stock/price changes

FindChips does offer API access but it is aimed at enterprise customers. No free tier is publicly documented. Less suitable than Nexar for a CLI tool because of limited public API access.

### OEMsecrets API

OEMsecrets offers an API for distributor pricing data. Less documentation available than Nexar. Worth monitoring but not a primary choice.

### Direct Distributor APIs (Current Approach)

Our existing Mouser, DigiKey, and JLCPCB integrations are already functional. For BOM management, the question is whether to:
1. **Use direct APIs** — more control, no aggregator dependency, but 3 separate queries per part and limited distributor coverage.
2. **Use Nexar** — single query, broader coverage, but adds a dependency and has usage limits.
3. **Hybrid** — use Nexar for initial BOM pricing/comparison, fall back to direct APIs for specific orders or when Nexar limits are hit.

**Recommendation:** Start with direct distributor APIs (already integrated), add Nexar as an optional aggregator for broader comparison. This keeps the tool functional without requiring a Nexar API key.

## 3. Proposed BOM JSON Schema

### Input BOM Format

The input BOM should be flexible enough to accept data from Altium, KiCad, or manual entry. It should be the minimal set of information needed to look up parts.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "BOM Input",
  "type": "object",
  "required": ["parts"],
  "properties": {
    "project_name": { "type": "string" },
    "revision": { "type": "string" },
    "board_quantity": {
      "type": "integer",
      "minimum": 1,
      "default": 1,
      "description": "Number of boards to build (multiplied by per-board quantity)"
    },
    "parts": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["designators", "quantity_per_board"],
        "properties": {
          "designators": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Reference designators, e.g. ['R1', 'R2', 'R5']"
          },
          "quantity_per_board": {
            "type": "integer",
            "minimum": 1,
            "description": "Number of this part per board (usually = len(designators))"
          },
          "manufacturer": { "type": "string" },
          "mpn": {
            "type": "string",
            "description": "Manufacturer part number"
          },
          "description": { "type": "string" },
          "value": {
            "type": "string",
            "description": "Component value, e.g. '100nF', '10k'"
          },
          "footprint": {
            "type": "string",
            "description": "Package/footprint name, e.g. '0402', 'SOIC-8'"
          },
          "distributor_pns": {
            "type": "object",
            "description": "Known distributor part numbers",
            "properties": {
              "mouser": { "type": "string" },
              "digikey": { "type": "string" },
              "lcsc": { "type": "string" }
            },
            "additionalProperties": { "type": "string" }
          },
          "alternatives": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "mpn": { "type": "string" },
                "manufacturer": { "type": "string" },
                "notes": { "type": "string" }
              }
            },
            "description": "Acceptable alternative parts"
          },
          "dnp": {
            "type": "boolean",
            "default": false,
            "description": "Do Not Place — excluded from pricing/ordering"
          }
        }
      }
    }
  }
}
```

### Output BOM Format (Priced BOM)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "BOM Output (Priced)",
  "type": "object",
  "properties": {
    "project_name": { "type": "string" },
    "revision": { "type": "string" },
    "board_quantity": { "type": "integer" },
    "total_quantity": {
      "type": "integer",
      "description": "Total parts across all boards"
    },
    "generated_at": {
      "type": "string",
      "format": "date-time"
    },
    "currency": { "type": "string", "default": "USD" },
    "summary": {
      "type": "object",
      "properties": {
        "total_cost_per_board": { "type": "number" },
        "total_cost_all_boards": { "type": "number" },
        "num_unique_parts": { "type": "integer" },
        "num_total_parts": { "type": "integer" },
        "num_distributors_used": { "type": "integer" },
        "parts_not_found": { "type": "integer" },
        "parts_out_of_stock": { "type": "integer" }
      }
    },
    "parts": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "designators": { "type": "array", "items": { "type": "string" } },
          "quantity_per_board": { "type": "integer" },
          "total_quantity": { "type": "integer" },
          "mpn": { "type": "string" },
          "manufacturer": { "type": "string" },
          "description": { "type": "string" },
          "value": { "type": "string" },
          "footprint": { "type": "string" },
          "dnp": { "type": "boolean" },
          "offers": {
            "type": "array",
            "description": "All available offers from all distributors",
            "items": {
              "type": "object",
              "properties": {
                "distributor": { "type": "string" },
                "sku": { "type": "string" },
                "stock": { "type": "integer" },
                "moq": { "type": "integer" },
                "order_multiple": { "type": "integer" },
                "lead_time_days": { "type": "integer" },
                "lifecycle": { "type": "string" },
                "packaging": { "type": "string" },
                "price_breaks": {
                  "type": "array",
                  "items": {
                    "type": "object",
                    "properties": {
                      "quantity": { "type": "integer" },
                      "unit_price": { "type": "number" },
                      "extended_price": { "type": "number" }
                    }
                  }
                },
                "unit_price_at_quantity": {
                  "type": "number",
                  "description": "Unit price for the required quantity"
                },
                "extended_price": {
                  "type": "number",
                  "description": "Total price for required quantity"
                },
                "in_stock": { "type": "boolean" },
                "url": { "type": "string" }
              }
            }
          },
          "best_offer": {
            "type": "object",
            "description": "The cheapest in-stock offer",
            "properties": {
              "distributor": { "type": "string" },
              "sku": { "type": "string" },
              "unit_price": { "type": "number" },
              "extended_price": { "type": "number" }
            }
          },
          "status": {
            "type": "string",
            "enum": ["ok", "out_of_stock", "not_found", "low_stock", "dnp"]
          }
        }
      }
    },
    "optimization": {
      "type": "object",
      "description": "Optimal distributor allocation",
      "properties": {
        "strategy": {
          "type": "string",
          "enum": ["cheapest", "fewest_distributors", "fastest_lead_time", "all_in_stock"]
        },
        "distributor_orders": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "distributor": { "type": "string" },
              "parts": {
                "type": "array",
                "items": {
                  "type": "object",
                  "properties": {
                    "mpn": { "type": "string" },
                    "sku": { "type": "string" },
                    "quantity": { "type": "integer" },
                    "unit_price": { "type": "number" },
                    "extended_price": { "type": "number" }
                  }
                }
              },
              "subtotal": { "type": "number" },
              "estimated_shipping": { "type": "number" }
            }
          }
        },
        "total_cost": { "type": "number" },
        "warnings": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    }
  }
}
```

### Design Rationale

- **Separate input/output schemas**: The input is what the user provides (or what's extracted from a schematic). The output is what the tool generates after querying distributors.
- **`designators` as array**: Matches how BOMs group identical parts. "R1, R2, R5" all use the same 10k resistor.
- **`alternatives`**: Allows the agent to specify acceptable substitutes (e.g., different manufacturers of the same capacitor).
- **`offers` array with all distributors**: Preserves full data for the agent to reason about tradeoffs.
- **`best_offer` shortcut**: Quick access to the cheapest in-stock option without scanning all offers.
- **`optimization` section**: The result of multi-distributor optimization, presented as per-distributor shopping lists.
- **`dnp` flag**: Important for prototype BOMs where some parts are intentionally not placed.

## 4. Multi-Distributor Optimization Approach

### Problem Statement

Given:
- N unique parts, each with offers from multiple distributors
- Each offer has: unit price (quantity-dependent), stock level, MOQ, order multiple
- Each distributor may charge shipping
- Goal: minimize total cost (parts + shipping) while ensuring all parts are in stock

This is a variant of the **weighted set cover problem** which is NP-hard in general. However, for typical BOM sizes (10-200 unique parts) and distributor counts (3-8), practical heuristics work well.

### Constraints

1. **Stock constraint**: Cannot order more than available stock from a distributor.
2. **MOQ constraint**: Must order at least MOQ quantity (may need to round up).
3. **Order multiple constraint**: Order quantity must be a multiple of the order increment.
4. **Quantity discount**: Price per unit decreases at higher quantities — sometimes it's cheaper to order more.
5. **Shipping cost**: Each additional distributor adds shipping cost (~$5-$20 per order).
6. **Minimum order value**: Some distributors have minimum order amounts.

### Proposed Algorithm

**Phase 1: Greedy allocation (good enough for most cases)**

```
1. For each part, query all distributors and collect offers
2. For each part, compute effective unit price at required quantity
   (considering MOQ — if MOQ > needed, unit price = (MOQ * price) / needed)
3. Score each offer: effective_unit_price + (shipping_penalty / num_parts_at_distributor)
4. Greedy assignment:
   a. Start by assigning every part to its cheapest in-stock offer
   b. Count distributors used
   c. For each part assigned to a "lonely" distributor (only 1-2 parts there):
      - Check if moving it to another distributor saves shipping cost
      - Move if net savings > 0
   d. Repeat until stable
5. Output per-distributor shopping lists
```

**Phase 2: ILP formulation (for power users, future work)**

```
Variables:
  x[i][j] = 1 if part i is bought from distributor j, 0 otherwise
  y[j] = 1 if distributor j is used, 0 otherwise

Minimize:
  sum(x[i][j] * price[i][j] * quantity[i]) + sum(y[j] * shipping[j])

Subject to:
  For each part i: sum(x[i][j] for all j) = 1  (each part from exactly one distributor)
  For each part i, distributor j: x[i][j] <= stock[i][j] >= quantity[i]  (stock available)
  For each i, j: x[i][j] <= y[j]  (if buying from j, j must be used)
  x[i][j] in {0, 1}, y[j] in {0, 1}
```

This can be solved exactly with a MIP solver (e.g., `good_lp` crate with CBC or HiGHS backend in Rust). For typical BOM sizes, this solves in milliseconds.

**Phase 3: Alternatives expansion**

When alternatives are specified, expand the search space:
- For each part with alternatives, query all alternatives from all distributors
- Include alternative offers in the optimization
- Report when an alternative is cheaper than the primary choice

### Practical Considerations

- **Caching**: Price/stock data should be cached for the session (prices change daily, not per-second). Use the existing `file_cache` module with a TTL of ~1 hour.
- **Rate limiting**: Mouser and DigiKey have rate limits. Query parts in parallel with backoff.
- **Fallback**: If a part is not found at any distributor, flag it and continue with partial results.
- **Attrition factor**: For production runs, add a configurable attrition/overage percentage (typically 1-5%) to account for assembly losses.

## 5. JLCPCB Assembly BOM Format

### Required Format

JLCPCB accepts BOM files in CSV, XLS, or XLSX format with these required columns:

| Column | Description | Example |
|--------|-------------|---------|
| Comment | Part value/specification | `100nF 50V X7R` |
| Designator | Reference designator(s) | `C1,C2,C5` or `C1 C2 C5` |
| Footprint | Package name | `0402`, `0805`, `SSOP-8` |
| LCSC Part # | LCSC/JLCPCB part number | `C49678` |

### Example CSV

```csv
Comment,Designator,Footprint,LCSC Part #
100nF 50V X7R,"C1,C2,C3,C5",0402,C307331
10uF 25V,"C4,C6",0805,C15850
10k,"R1,R2,R3",0402,C25744
ESP32-C6-WROOM-1U-N8,U1,ESP32-C6-WROOM-1U,C5765514
```

### Important Rules

1. **Designators are case-insensitive**: JLCPCB converts all letters to uppercase.
2. **Multiple designators**: Comma-separated within the same cell, or space-separated. Must be quoted if comma-separated in CSV.
3. **LCSC Part Number**: Format is `C` followed by digits (e.g., `C49678`). Including this guarantees 100% part matching accuracy.
4. **Assembly categories matter for cost**:
   - **Basic**: No extra setup fee, these are parts JLCPCB keeps in stock on their feeders.
   - **Preferred**: Extended parts that are popular, slightly cheaper than generic extended.
   - **Extended**: Each unique extended part adds a setup fee (~$3 per part type).
5. **Assembly process**: SMT parts are standard; through-hole parts cost extra.
6. **Minimum order**: Each LCSC part has its own MOQ, separate from JLCPCB board MOQ.

### Generation from Our BOM Schema

To generate a JLCPCB-compatible BOM from our output schema:

```
For each part in bom.parts where !dnp:
  comment = part.value or part.description
  designator = part.designators.join(",")
  footprint = part.footprint
  lcsc_pn = part.distributor_pns.lcsc or lookup via jlcpcb search
```

This should be a built-in export format: `datasheet bom export --format jlcpcb <bom.json>`.

## 6. Open-Source Tools to Reference

### KiCost

- **Repository**: https://github.com/hildogjr/KiCost
- **Language**: Python
- **What it does**: Takes a KiCad BOM XML (or Altium, Eagle, Proteus, CSV) and generates a spreadsheet with pricing from multiple distributors.
- **Supported distributors**: Arrow, DigiKey, Mouser, Newark, Farnell, RS, TME. Uses both web scraping and the Kitspace PartInfo API (which itself aggregates from Octopart).
- **Architecture**: Modular distributor backends, parallel fetching, spreadsheet output with per-distributor pricing columns.
- **Output**: XLSX spreadsheet with color-coded best prices, per-distributor columns, quantity-adjusted pricing, and cut-and-paste order lists.
- **Relevance**: Good reference for which data fields matter and how to present comparative pricing. However, its web scraping approach is fragile and its Python/spreadsheet-centric design doesn't fit our JSON-pipeline approach.
- **Key lesson**: KiCost's biggest user complaint is broken scraping. API-first design is essential.

### IndaBOM

- **Repository**: https://github.com/mpkasp/indabom (django-bom)
- **Language**: Python/Django
- **What it does**: Web-based indented BOM management with Octopart and Mouser integration for cost estimates.
- **Relevance**: More of a PLM tool than a CLI BOM pricer. Useful reference for BOM data modeling (indented BOMs with sub-assemblies) but overkill for our use case.

### PartKeepr

- **What it does**: Inventory management for electronic parts. Tracks what you have in stock, not what you need to buy.
- **Relevance**: Different problem domain (inventory vs. procurement). Not directly applicable.

### InvenTree

- **Repository**: https://github.com/inventree/InvenTree
- **Language**: Python/Django
- **What it does**: Open-source inventory management with supplier integration.
- **Relevance**: Has a supplier panel with pricing integration. Good reference for data modeling of supplier relationships.

### Bomist

- **What it does**: BOM management with pricing from multiple distributors.
- **Relevance**: Commercial tool, not open source. UI-focused, not API-focused.

## 7. Recommended Phased Implementation

### Phase A: Single-Part Pricing Query (MVP)

**Goal**: `datasheet bom price <mpn> --quantity 100`

Returns pricing and stock for a single MPN across all integrated distributors (Mouser, DigiKey, JLCPCB). This is the building block.

**Implementation:**
1. Add a `bom` top-level subcommand to the CLI.
2. Add a `price` subcommand that takes an MPN and quantity.
3. Query Mouser, DigiKey, and JLCPCB in parallel for the given MPN.
4. Normalize responses into a common `PartOffer` struct:
   ```rust
   struct PartOffer {
       distributor: String,
       sku: String,
       mpn: String,
       manufacturer: String,
       stock: i64,
       moq: i32,
       order_multiple: i32,
       lead_time_days: Option<i32>,
       lifecycle: Option<String>,
       packaging: Option<String>,
       price_breaks: Vec<PriceBreak>,
       unit_price_at_qty: f64,  // computed for requested quantity
       url: Option<String>,
   }
   ```
5. Display results as a comparison table, or output as JSON with `--json`.
6. Add parsing utilities for Mouser's string-based stock/price fields.

**Effort**: Small. Mostly glue code over existing integrations.

### Phase B: BOM Input/Output and Batch Pricing

**Goal**: `datasheet bom price --input bom.json --quantity 10 --output priced-bom.json`

Takes a BOM file, queries all parts, produces a priced BOM.

**Implementation:**
1. Define the input/output JSON schemas (as described in section 3).
2. Add BOM file parsing (JSON input, possibly CSV input for convenience).
3. Batch query: iterate over unique parts, query each across distributors.
   - DigiKey supports batch (50 parts/request) — use it.
   - Mouser and JLCPCB are one-at-a-time — parallelize with rate limiting.
4. Compute `unit_price_at_quantity` for each offer given the required quantity.
5. Identify `best_offer` per part.
6. Compute BOM totals.
7. Output priced BOM as JSON and/or formatted table.

**Effort**: Medium. The normalization layer is the main work.

### Phase C: JLCPCB Assembly Export

**Goal**: `datasheet bom export --format jlcpcb --input bom.json --output jlcpcb-bom.csv`

Generates a JLCPCB-compatible BOM CSV from the priced BOM.

**Implementation:**
1. For each part in the BOM, look up the LCSC part number:
   - Use `distributor_pns.lcsc` if provided in the input.
   - Otherwise, search JLCPCB by MPN using existing `jlcpcb_search`.
2. Emit CSV with columns: Comment, Designator, Footprint, LCSC Part #.
3. Warn about parts not found on JLCPCB, extended parts (extra cost), and out-of-stock parts.
4. Optionally output assembly category analysis: count basic vs. extended parts, estimate setup fees.

**Effort**: Small once Phase B is done.

### Phase D: Multi-Distributor Optimization

**Goal**: `datasheet bom optimize --input priced-bom.json --output optimized-bom.json`

Or integrated into the pricing step:
`datasheet bom price --input bom.json --optimize --quantity 10`

**Implementation:**
1. Implement the greedy allocation algorithm (Phase 1 from section 4).
2. Add configurable shipping cost estimates per distributor.
3. Output per-distributor shopping lists with subtotals.
4. Warn about:
   - Parts only available from one distributor
   - Parts where MOQ significantly exceeds needed quantity
   - Out-of-stock parts
   - End-of-life parts

**Effort**: Medium. Algorithm is straightforward, but needs good UX for the output.

### Phase E: Nexar/Octopart Integration (Optional Aggregator)

**Goal**: `datasheet bom price --input bom.json --nexar --quantity 10`

Use Nexar API to get pricing from all distributors in a single query.

**Implementation:**
1. Add `nexar` module with OAuth2 authentication and GraphQL client.
2. Implement `supMultiMatch` queries with seller/offer/pricing fields.
3. Map Nexar response to our `PartOffer` struct.
4. Fall back to direct APIs if Nexar fails or quota is exhausted.

**Effort**: Medium. New API integration + GraphQL client.

**When to build**: Only if users need pricing from distributors beyond Mouser/DigiKey/JLCPCB (Arrow, Newark, Farnell, RS, etc.). The free tier (100 parts/month) is very limited; the Standard tier (2,000 parts/month) loses lead time data. May not be worth the dependency for most users.

### Phase F: ILP Solver for Optimal Allocation (Future)

**Goal**: Replace greedy allocation with exact optimization using integer linear programming.

**Implementation:**
1. Add `good_lp` crate dependency with HiGHS or CBC solver backend.
2. Formulate the BOM allocation problem as described in section 4.
3. Solve and extract allocation.

**Effort**: Small code change if the data structures from Phase D are in place. The solver does the hard work.

**When to build**: Only if the greedy algorithm produces noticeably suboptimal results for real-world BOMs. For most hobby/small-production runs with 3 distributors, greedy is sufficient.

## 8. CLI Command Structure (Proposed)

```
datasheet bom price <mpn> [--quantity N]           # Single part pricing
datasheet bom price --input <bom.json> [--quantity N] [--optimize]  # Full BOM pricing
datasheet bom export --format jlcpcb --input <bom.json> -o <output.csv>  # JLCPCB export
datasheet bom export --format csv --input <bom.json> -o <output.csv>     # Generic CSV
datasheet bom check --input <bom.json>             # Stock check only (fast)
datasheet bom optimize --input <priced-bom.json>   # Re-optimize existing priced BOM
```

## 9. Open Questions

1. **Altium BOM import**: Should we support reading Altium's native BOM export format directly? Altium exports CSV/XLS BOMs with configurable columns. We could add a `--format altium` parser.

2. **Caching strategy**: Price/stock data changes frequently. What TTL? Suggested: 1 hour for stock, 24 hours for pricing (prices change daily in batch updates, not real-time). Lead time: 1 week (changes slowly).

3. **Currency**: Should we normalize everything to USD, or support multi-currency? Most distributors price in USD for the US market, but Farnell/RS price in GBP/EUR.

4. **Nexar vs. direct APIs**: The Nexar free tier is very limited. Is it worth integrating, or should we focus on making the direct distributor integrations excellent? Direct APIs have no per-query cost.

5. **Spreadsheet output**: KiCost produces XLSX spreadsheets which are popular with procurement teams. Should we support XLSX output, or stick to JSON + CSV? XLSX would require an additional crate dependency.

6. **Authentication management**: With potentially 4+ API keys (Mouser, DigiKey, Nexar, future distributors), should we add a `datasheet config` command for managing credentials? Or continue relying on env vars?

7. **Order placement**: DigiKey and Mouser APIs support cart/order placement. Should we add `datasheet bom order` to add parts to carts on each distributor? This would be very powerful but adds complexity and risk.
