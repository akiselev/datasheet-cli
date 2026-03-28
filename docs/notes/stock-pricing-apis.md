# Research: Stock, Pricing, and Availability APIs for Electronics Distributors

**Date:** 2026-03-17
**Goal:** Evaluate what APIs are available for querying electronic component stock levels, pricing, lead times, and lifecycle status, so an LLM agent can verify part availability early in the design process before committing to a part.

---

## 1. Existing Integrations in datasheet-cli

### 1.1 Mouser (already integrated)

**Source:** `src/mouser.rs`
**Base URL:** `https://api.mouser.com/api/v1`
**Endpoints used:**
- `POST /search/keyword` -- keyword search
- `POST /search/partnumber` -- exact part number search

**Stock/pricing data already fetched but only partially exposed:**

The Mouser API already returns comprehensive stock and pricing data in every search response. The `Part` struct in the code deserializes all of these fields:

| Field | Type | Status |
|-------|------|--------|
| `AvailabilityInStock` | String (e.g. "1,234 In Stock") | **Fetched and displayed** |
| `AvailabilityOnOrder` | String or Array of objects | **Fetched and displayed** |
| `LeadTime` | String (e.g. "6 weeks") | **Fetched and displayed** |
| `LifecycleStatus` | String | **Fetched and displayed** |
| `Min` | String (minimum order qty) | **Fetched and displayed** |
| `Mult` | String (order multiple) | **Fetched and displayed** |
| `PriceBreaks` | Array of `{Quantity, Price, Currency}` | **Fetched and displayed** |
| `SuggestedReplacement` | String | Fetched, not displayed |
| `Reeling` | bool | Fetched, not displayed |
| `ROHSStatus` | String | Fetched, displayed |
| `AlternatePackagings` | JSON value | Fetched, not displayed |

**Conclusion:** Mouser already provides everything we need. The `part` and `search` commands already display stock, pricing, lead time, MOQ, lifecycle status, and price breaks. No new endpoints are needed. The data just needs to be surfaced in a structured way for machine consumption (the `--json` flag already does this).

**Authentication:** Single API key via `MOUSER_API_KEY` env var. Free to obtain from mouser.com.

**Rate limits:** Not explicitly documented in public materials. The API guide references rate limiting but does not publish specific numbers. Third-party libraries note that rate limiting is enforced and handled by waiting.

**Additional V2 endpoints available but not yet used:**
- `KeywordAndManufacturerSearch` -- search with manufacturer filter, supports pagination
- `PartNumberAndManufacturerSearch` -- more precise part matching

---

### 1.2 DigiKey (already integrated)

**Source:** `src/digikey.rs`
**Base URL:** `https://api.digikey.com`
**API version:** Product Information V4
**Endpoints used:**
- `POST /products/v4/search/keyword` -- keyword search
- `GET /products/v4/search/{partNumber}/productdetails` -- exact part lookup

**Stock/pricing data already fetched but only partially exposed:**

The `Product` struct deserializes these fields:

| Field | Type | Status |
|-------|------|--------|
| `QuantityAvailable` | i32 | **Fetched and displayed** |
| `ManufacturerPublicQuantity` | i32 | **Fetched and displayed** |
| `MinimumOrderQuantity` | i32 | **Fetched and displayed** |
| `StandardPricing` | Array of `{BreakQuantity, UnitPrice, TotalPrice}` | **Fetched and displayed** |
| `UnitPrice` | f64 | Fetched, not separately displayed |
| `PartStatus` | String (e.g. "Active", "Obsolete") | **Fetched and displayed** |
| `LeadStatus` | String | **Fetched and displayed** |
| `Packaging` | Object with `Value` field | **Fetched and displayed** |
| `RoHSStatus` | String | **Fetched and displayed** |

**Conclusion:** Like Mouser, DigiKey already provides stock, pricing, and lifecycle data in the existing search/detail responses. The `--json` output already includes all of it. No new endpoints required for basic availability checking.

**Additional pricing endpoints available but not yet used:**

1. **`GET /products/v4/search/{productNumber}/pricing`** (ProductPricing)
   - Returns pricing across all product variations (tape & reel, cut tape, etc.)
   - Includes `StandardPricing[]` and `MyPricing[]` (customer-specific)
   - Returns `ManufacturerLeadWeeks`, `IsDiscontinued`, `IsObsolete`, `IsEndOfLife`, `NormallyStocking`
   - Limit parameter: max 10 results
   - Useful for comparing packaging options and getting customer-specific pricing

2. **`GET /products/v4/search/{productNumber}/pricingbyquantity/{requestedQuantity}`** (PricingOptionsByQuantity)
   - Given a part number and desired quantity, returns up to 4 pricing scenarios:
     - **Exact**: price at the requested quantity
     - **MinimumOrderQuantity**: price if quantity is increased to MOQ
     - **MaxOrderQuantity**: price if quantity exceeds max
     - **BetterValue**: price if quantity is increased to a standard package boundary (often cheaper per unit)
   - Very useful for BOM costing -- lets the agent find the best price for a specific build quantity

**Authentication:** OAuth2 client credentials flow. Requires `DIGIKEY_CLIENT_ID` and `DIGIKEY_CLIENT_SECRET` env vars. Free to register at developer.digikey.com.

**Rate limits:**
- Burst limit: 120 requests per minute
- Daily limit: tracked via `X-RateLimit-Limit` header (specific number not publicly documented, but forum posts suggest ~1000/day for default tier)
- 429 status code when exceeded, with `Retry-After` header
- Rate limit increase available by contacting DigiKey

---

### 1.3 JLCPCB/LCSC (already integrated)

**Source:** `src/jlcpcb.rs`
**Endpoints used:**
- `POST https://jlcpcb.com/api/overseas-pcb-order/v1/shoppingCart/smtGood/selectSmtComponentList/v2` -- search
- `GET https://cart.jlcpcb.com/shoppingCart/smtGood/getComponentDetail?componentCode={code}` -- part detail

**Stock/pricing data already fetched and exposed:**

| Field | Type | Status |
|-------|------|--------|
| `stock_count` | i64 | **Fetched and displayed** |
| `component_prices` / `prices` | Array of `{start_number, product_price}` | **Fetched and displayed** |
| `component_library_type` | String (basic/expand) | **Fetched and displayed** (as category) |
| `preferred_component_flag` | bool | **Fetched, used for category** |
| `min_purchase_num` | i32 | **Fetched and displayed** |
| `assembly_process` | String (SMT/THT) | **Fetched and displayed** |

**Conclusion:** JLCPCB integration already returns stock and pricing. No lead time data is available through this API (JLCPCB stocks parts in their warehouse; if in stock, it ships with the PCB assembly order). The key missing data point for JLCPCB is whether the part is "basic" (no additional fee), "preferred" (small fee), or "extended" (setup fee per unique part) -- but this is already handled via the `category` field.

**Authentication:** None required for the JLCPCB cart API endpoints currently used.

**Rate limits:** Not documented for the undocumented JLCPCB API. These are internal endpoints discovered by reverse engineering, so they could change or impose limits without notice.

---

### 1.4 LCSC (official API, not yet integrated)

LCSC is a separate entity from JLCPCB (though related). LCSC has an **official public API** with proper documentation.

**Base URL:** `https://wmsc.lcsc.com` (inferred from documentation)
**Documentation:** https://www.lcsc.com/docs/openapi/index.html

**Endpoints:**

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/rest/wmsc2agent/category` | GET | All categories |
| `/rest/wmsc2agent/brand` | GET | All manufacturers |
| `/rest/wmsc2agent/category/product/{category_id}` | GET | Products by category with stock & pricing |
| `/rest/wmsc2agent/product/info/{product_number}` | GET | Full part details with pricing tiers, stock per warehouse, attributes |
| `/rest/wmsc2agent/search/product` | GET | Keyword search (max 30 results/page) |

**Data returned:**
- Stock quantities across multiple warehouses (Jiangsu, Zhuhai, Hong Kong)
- Pricing tiers by quantity
- MOQ, availability status, pre-sale status
- Full product attributes, datasheets, images
- Currency support: USD, CNY, EUR, HKD

**Authentication:** API key + HMAC signature. Requires:
- `key` -- API key
- `nonce` -- 16-character random string
- `signature` -- calculated signature
- `timestamp` -- request timestamp

Must apply for API access via LCSC account.

**Rate limits:** 1,000 searches/day, 200 searches/minute. Higher limits available upon request.

---

## 2. Aggregator APIs (not yet integrated)

### 2.1 Nexar (Octopart) -- RECOMMENDED

**Overview:** Nexar is the company behind Octopart. The old Octopart REST API has been replaced by the Nexar GraphQL API. This is the single most powerful option because it aggregates data from 100+ distributors in one query.

**API type:** GraphQL
**Endpoint:** `https://api.nexar.com/graphql/`
**Identity service:** `https://identity.nexar.com/connect/token`

**Data available per query:**
- Stock levels per distributor (`inventoryLevel`, `totalAvail`)
- Price breaks per distributor (`prices[]{quantity, price, currency, convertedPrice, convertedCurrency}`)
- Median price at volume (e.g. `medianPrice1000`)
- Lead times (`factoryLeadDays`)
- MOQ (`moq`)
- Packaging type (`packaging`)
- Part lifecycle/category info
- Seller/distributor names and IDs
- Currency conversion (specify `currency` parameter)
- Country-specific availability (specify `country` parameter)

**Key queries:**

```graphql
# Search by MPN with full pricing and stock
query partAvailability {
  supSearchMpn(q: "STM32F407VET6", country: "US", currency: "USD", limit: 5) {
    hits
    results {
      part {
        mpn
        name
        totalAvail
        medianPrice1000 { quantity, convertedPrice, convertedCurrency }
        sellers {
          company { name }
          offers {
            inventoryLevel
            moq
            factoryLeadDays
            packaging
            prices { quantity, price, currency, convertedPrice, convertedCurrency }
          }
        }
      }
    }
  }
}

# Batch match multiple MPNs
query bomPricing {
  supMultiMatch(
    queries: [
      {mpn: "STM32F407VET6", limit: 1},
      {mpn: "LM1117IMP-3.3", limit: 1},
      {mpn: "GRM188R71C104KA01D", limit: 1}
    ]
  ) {
    hits
    parts {
      mpn
      totalAvail
      sellers {
        company { name }
        offers {
          inventoryLevel
          prices { quantity, price, currency }
        }
      }
    }
  }
}
```

**Authentication:** OAuth2 client credentials flow.
1. Register at nexar.com, create an application
2. Get `client_id` and `client_secret`
3. POST to `https://identity.nexar.com/connect/token` with `grant_type=client_credentials` and `scope=supply.domain`
4. Access token valid for 24 hours

**Pricing tiers:**

| Plan | Monthly Parts Limit | Price | Key Features |
|------|---------------------|-------|--------------|
| Evaluation | 100 (lifetime) | Free | All features (pricing, stock, lead time, lifecycle, datasheets, tech specs) |
| Standard | 2,000/month | Paid (contact) | Pricing, stock, images, descriptions only |
| Pro | 15,000/month | Paid (contact) | Adds lead time, lifecycle, datasheets, tech specs |
| Enterprise | Custom | Custom | Full access including ECAD models, similar parts |

**Part limit note:** Limits are based on the number of *parts returned*, not queries made. A query returning 10 parts costs 10 against your limit. Some queries (e.g. category lookups) are free.

**Rate limits:** Not explicitly documented per-request, but token refresh is rate-limited to protect the identity server.

**Why this is the top recommendation:**
- One query gets stock and pricing across Mouser, DigiKey, Farnell, Arrow, TME, RS Components, and 100+ other distributors simultaneously
- GraphQL lets you request exactly the fields you need
- `supMultiMatch` enables batch BOM pricing in a single request
- Currency conversion built in
- Lead time data included

---

### 2.2 Element14 / Farnell / Newark

**Overview:** Element14 (parent of Farnell and Newark) offers a Product Search API covering their inventory of 1.3M+ products.

**API type:** REST (also SOAP beta)
**Documentation:** https://partner.element14.com/docs

**Data returned:**
- Real-time stock levels
- Real-time pricing (standard and contract/customer-specific)
- Product specifications
- Datasheet URLs

**Authentication:** API key obtained after registration at partner.element14.com. Contract pricing requires additional CustomerID and Secret Key from sales rep.

**Rate limits:** Not publicly documented.

**Relevance:** Useful for European designs where Farnell is a primary supplier. Lower priority than Nexar since Nexar already aggregates Farnell data.

---

### 2.3 TME (Transfer Multisort Elektronik)

**Overview:** Major European distributor with 700,000+ products. Full-featured API.

**API type:** REST (POST only)
**Base URL:** `https://api.tme.eu/`
**Documentation:** https://api-doc.tme.eu/

**Key endpoints:**

| Endpoint | Purpose | Limit |
|----------|---------|-------|
| `/Products/Search` | Text/category search | 20 items/page |
| `/Products/GetPrices` | Price tiers by quantity | Max 50 symbols per request |
| `/Products/GetStocks` | Current stock quantities | Max 50 symbols per request |
| `/Products/GetPricesAndStocks` | Combined pricing + stock | Max 50 symbols per request |
| `/Products/GetDeliveryTime` | Estimated delivery times | Max 50 symbols per request |
| `/Products/GetParameters` | Technical specifications | Max 50 symbols per request |
| `/Products/GetSimilarProducts` | Alternative parts | Max 50 symbols per request |

**Data returned:**
- Price tiers (net/gross, with VAT info)
- Stock quantities with units
- Delivery status codes: `DS_AVAILABLE_IN_STOCK`, `DS_DELIVERY_NEEDS_CONFIRMATION`, etc.
- Estimated delivery weeks and dates (ISO 8601)
- Product status flags: `NEW`, `SALE`, `PROMOTED`, `NOT_IN_OFFER`, etc.
- MOQ and order multiplicity
- Multi-currency support: BGN, CZK, EUR, GBP, HUF, PLN, RON, USD

**Authentication:** Token-based (HMAC-SHA1 signature). 50-character private token tied to customer account.

**Rate limits:**
- General endpoints: 10 requests/second per token
- Pricing/stock/delivery endpoints: 2 requests/second per token
- HTTP 429 with `Retry-After` header when exceeded

**Relevance:** Important for European BOM sourcing. Has a very clean batch API (`GetPricesAndStocks` for up to 50 parts at once). Lower priority than Nexar for single-part checks but could be useful as a direct source.

---

### 2.4 PartFuse

**Overview:** Lightweight aggregator for Mouser, DigiKey, and TME pricing/stock. Hosted on RapidAPI.

**API type:** REST (via RapidAPI)
**Documentation:** https://github.com/PartFuse/partfuse-examples

**Features:**
- Unified search across Mouser, DigiKey, TME
- Quantity-based pricing
- Bulk BOM pricing via JSON input
- Real-time stock levels

**Limitations:**
- Information only (no ordering)
- Data accuracy is "best-effort"
- Third-party dependency (RapidAPI)
- Covers only 3 distributors vs Nexar's 100+

**Relevance:** Low priority. Nexar provides better coverage. Only useful if Nexar pricing is prohibitive.

---

### 2.5 OEMSecrets

**Overview:** Another aggregator API covering multiple distributors.

**API type:** REST
**Documentation:** https://www.oemsecrets.com/api

**Relevance:** Low priority. Limited public documentation. Nexar is the better-known and better-documented aggregator.

---

## 3. Summary: What Each API Provides

| Feature | Mouser | DigiKey | JLCPCB | LCSC (official) | Nexar | TME | Element14 |
|---------|--------|---------|--------|-----------------|-------|-----|-----------|
| Stock level | Yes | Yes | Yes | Yes (per warehouse) | Yes (per distributor) | Yes | Yes |
| Price breaks | Yes | Yes | Yes | Yes | Yes (per distributor) | Yes | Yes |
| Lead time | Yes | Partial* | No | No | Yes (`factoryLeadDays`) | Yes (delivery weeks) | Unknown |
| MOQ | Yes | Yes | Yes | Yes | Yes | Yes | Unknown |
| Lifecycle status | Yes | Yes (`PartStatus`) | No | No | Yes | Partial (flags) | Unknown |
| Batch query | No** | No** | No | No | Yes (`supMultiMatch`) | Yes (50/request) | Unknown |
| Currency conversion | No | Yes (header) | No | Yes | Yes (param) | Yes (param) | Unknown |
| Multi-distributor | No | No | No | No | **Yes (100+)** | No | No |
| Auth required | API key | OAuth2 | None | API key + HMAC | OAuth2 | HMAC token | API key |
| Free tier | Yes | Yes | Yes | Apply | 100 parts lifetime | Yes | Yes |

*DigiKey returns `LeadStatus` and `ManufacturerLeadWeeks` in pricing endpoints but not in the basic search/detail response currently used.
**Mouser and DigiKey support batch via repeated calls but have no single batch endpoint.

---

## 4. Data Already Fetched But Not Exposed as a Dedicated Command

Both Mouser and DigiKey already return stock/pricing data in their search and part-detail responses. The current CLI exposes this data through `search --json` and `part --json`, but there is no dedicated `stock` or `pricing` subcommand optimized for quick availability checks.

**Specific fields fetched but underutilized:**

**Mouser:**
- `SuggestedReplacement` -- useful for EOL parts
- `AlternatePackagings` -- useful for assembly optimization
- `Reeling` -- whether tape & reel is available

**DigiKey:**
- The `ProductPricing` and `PricingOptionsByQuantity` endpoints are not used at all. These provide richer pricing data than the basic `ProductDetails` endpoint, including:
  - Pricing across all product variations (cut tape, tape & reel, etc.)
  - Customer-specific pricing (`MyPricing`)
  - Smart quantity suggestions (MOQ, better-value quantities)
  - `ManufacturerLeadWeeks`, `IsDiscontinued`, `IsObsolete`, `IsEndOfLife` flags

---

## 5. Recommended Implementation Approach

### Phase 1: Surface existing data (low effort, high value)

Add a `stock` subcommand to Mouser, DigiKey, and JLCPCB that outputs a focused availability summary:

```
datasheet mouser stock <part-number> [--json]
datasheet digikey stock <part-number> [--json]
datasheet jlcpcb stock <part-number> [--json]
```

Output format (human-readable):
```
STM32F407VET6 (STMicroelectronics)
  Status: Active
  In Stock: 12,345
  Lead Time: 12 weeks
  MOQ: 1 | Multiple: 1
  Pricing:
      1+ : $12.35 USD
     10+ : $11.22 USD
    100+ : $9.45 USD
   1000+ : $8.12 USD
```

JSON output would be a normalized struct:
```json
{
  "mpn": "STM32F407VET6",
  "manufacturer": "STMicroelectronics",
  "distributor": "mouser",
  "lifecycle_status": "Active",
  "stock": 12345,
  "lead_time": "12 weeks",
  "moq": 1,
  "order_multiple": 1,
  "price_breaks": [
    {"quantity": 1, "unit_price": 12.35, "currency": "USD"},
    {"quantity": 10, "unit_price": 11.22, "currency": "USD"}
  ]
}
```

This requires **zero new API calls** -- the data is already in the search/part responses. It just needs a new subcommand that formats the output for machine consumption.

### Phase 2: Add DigiKey pricing endpoints (medium effort, high value)

Implement the `ProductPricing` and `PricingOptionsByQuantity` endpoints:

```
datasheet digikey pricing <part-number> [--json]
datasheet digikey pricing <part-number> --quantity 500 [--json]
```

This adds:
- Pricing across packaging variants
- Smart quantity suggestions (MOQ boundary, better-value quantity)
- `ManufacturerLeadWeeks`, obsolescence flags
- Customer-specific pricing (if account ID is configured)

### Phase 3: Add Nexar/Octopart integration (higher effort, highest value)

Add a `nexar` subcommand:

```
datasheet nexar search <query> [--json]
datasheet nexar stock <mpn> [--country US] [--currency USD] [--json]
datasheet nexar bom-price <bom.json> [--quantity 100] [--json]
```

The `bom-price` command would be the killer feature: given a JSON BOM file, it queries `supMultiMatch` for all parts in one request and returns a full pricing/availability report.

**Environment variables needed:**
- `NEXAR_CLIENT_ID`
- `NEXAR_CLIENT_SECRET`

**Implementation notes:**
- Use `ureq` for HTTP (consistent with existing code)
- GraphQL queries can be sent as POST with JSON body `{"query": "...", "variables": {...}}`
- Token caching: store OAuth token in `~/.cache/datasheet-cli/nexar/` with 24h TTL (same pattern as SnapEDA cache)
- Part limit tracking: log how many parts have been consumed against the plan limit

### Phase 4: Add LCSC official API (optional, medium effort)

Replace or supplement the undocumented JLCPCB cart API with the official LCSC API for more reliable, documented access. Key advantage: per-warehouse stock levels and proper authentication.

### Phase 5: Unified availability check (highest value)

Add a top-level command that queries all configured distributors:

```
datasheet check-availability <mpn> [--json]
datasheet bom-check <bom.json> [--json]
```

This would:
1. Query Mouser, DigiKey, and JLCPCB in parallel (using existing integrations)
2. Optionally query Nexar for additional distributor coverage
3. Return a consolidated availability report

```json
{
  "mpn": "STM32F407VET6",
  "manufacturer": "STMicroelectronics",
  "total_stock": 45678,
  "best_price_1": {"distributor": "digikey", "price": 12.05, "currency": "USD"},
  "best_price_100": {"distributor": "mouser", "price": 9.32, "currency": "USD"},
  "sources": [
    {"distributor": "mouser", "stock": 12345, "lead_time": "12 weeks", "lifecycle": "Active", "price_breaks": [...]},
    {"distributor": "digikey", "stock": 23456, "lead_time": null, "lifecycle": "Active", "price_breaks": [...]},
    {"distributor": "jlcpcb", "stock": 9877, "category": "extended", "price_breaks": [...]}
  ],
  "risk_assessment": "low"  // based on stock levels and number of sources
}
```

---

## 6. Risk Assessment Logic

For the LLM agent use case, the most valuable output is a simple risk assessment:

| Risk Level | Criteria |
|------------|----------|
| **Low** | In stock at 2+ distributors, lifecycle Active, total stock > 10x needed quantity |
| **Medium** | In stock at 1 distributor, or total stock < 10x needed, or lead time > 8 weeks |
| **High** | Out of stock everywhere, or lifecycle NRND/Obsolete/EOL, or single-source with low stock |
| **Critical** | Part is obsolete/discontinued with no stock anywhere |

This would let the agent make quick go/no-go decisions during Phase 1 (Part Selection) without the human needing to manually check distributor websites.

---

## 7. Open Questions

1. **Nexar pricing:** The Standard and Pro plan costs are not publicly listed. Need to contact Nexar sales or sign up to see pricing. The free Evaluation tier (100 lifetime parts) is too small for real use; Standard (2,000/month) would likely be needed.

2. **Mouser API rate limits:** Not publicly documented. Need to empirically test or contact Mouser automation services (automation.services@mouser.com).

3. **DigiKey daily rate limit:** The exact number is tracked via headers but not published. Forum posts suggest ~1,000/day for default apps, with increases available upon request.

4. **LCSC API access:** Requires application and approval. Need to apply and test whether the official API provides meaningfully different data from the undocumented JLCPCB endpoints.

5. **Arrow API:** Arrow Electronics does not appear to have a public API. Their data is available through Nexar/Octopart.

6. **RS Components API:** RS Components (now RS Group, including Allied Electronics) -- no public API found. Available through Nexar/Octopart.
