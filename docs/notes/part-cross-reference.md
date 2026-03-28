# Research: Part Cross-Reference and Alternative Part Finding APIs

**Date:** 2026-03-17
**Goal:** Evaluate APIs and data sources for finding alternative/substitute electronic components when a preferred part is unavailable (out of stock, EOL, too expensive). This capability is critical for the LLM agent during Phase 1 (Part Selection) when a chosen part turns out to be unavailable or when the agent needs to present multiple sourcing options.

**Related research:** See `stock-pricing-apis.md` for distributor availability/pricing APIs (Mouser, DigiKey, JLCPCB, Nexar, TME, etc.).

---

## 1. Levels of "Compatibility"

Before evaluating data sources, it is important to define what "alternative" means. There are distinct levels, and the agent needs to understand which level applies:

### 1.1 Pin-Compatible Drop-In Replacement
- Identical pinout (same pin count, same pin functions, same package)
- Same or better electrical specifications
- Can be placed on the same PCB footprint without board changes
- Example: LM1117-3.3 from TI vs AMS1117-3.3 from AMS -- same SOT-223 pinout, same function, interchangeable

### 1.2 Functionally Equivalent (Different Pinout or Package)
- Same function and similar specifications
- Different pinout or package -- requires schematic/footprint changes
- Example: AMS1117-3.3 (SOT-223) vs AP2112K-3.3 (SOT-23-5) -- both are 3.3V LDOs but different packages/pinouts

### 1.3 Parametrically Similar (Same Category, Similar Specs)
- Same component category (e.g. "3.3V LDO regulator" or "100uF 16V electrolytic capacitor")
- Similar key parameters (voltage, current, tolerance, package size)
- May differ in pinout, package, or secondary specs
- Requires full schematic review to ensure compatibility

### 1.4 Application Equivalent (Same Function, Different Approach)
- Solves the same design problem but may use a different topology
- Example: replacing an LDO with a buck converter for better efficiency
- Requires significant design changes -- beyond what an automated cross-reference can suggest

**For automated cross-reference, levels 1 and 2 are the most valuable.** Level 3 is achievable through parametric search (already partially supported). Level 4 requires human engineering judgment.

---

## 2. Available APIs and Data Sources

### 2.1 Nexar/Octopart API -- `similarParts` Field

**Status:** Best available API for cross-reference. Already partially researched in `stock-pricing-apis.md`.

**API details:**
- **Type:** GraphQL
- **Endpoint:** `https://api.nexar.com/graphql/`
- **Auth:** OAuth2 client credentials (client_id + client_secret -> bearer token, 24h TTL)
- **Identity service:** `https://identity.nexar.com/connect/token`

**The `similarParts` field on the `Part` type:**

The Nexar GraphQL schema includes a `similarParts` field on the `Part` object. This returns a list of `Part` objects that Octopart considers similar. From the C# type definitions in the official Nexar example repository (`NexarDeveloper/nexar-first-supply-query`), the `Part` type includes:

```
Part {
  name: String
  mpn: String
  shortDescription: String
  manufacturer: Company { name, homepageUrl }
  category: Category { name }
  medianPrice1000: Price { quantity, convertedPrice, convertedCurrency }
  bestDatasheet: Datasheet { url, createdAt }
  specs: [Spec { attribute: Attribute { shortname }, value }]
  estimatedFactoryLeadDays: Int
  similarParts: [Part]          // <-- key field for cross-reference
  sellers: [Seller { ... }]
}
```

Each similar part is itself a full `Part` object, so you can query its MPN, manufacturer, specs, pricing, stock, and lifecycle status in a single GraphQL query.

**Example query:**

```graphql
query FindAlternatives($mpn: String!) {
  supSearchMpn(q: $mpn, limit: 1) {
    results {
      part {
        mpn
        manufacturer { name }
        category { name }
        shortDescription
        specs {
          attribute { shortname }
          value
        }
        similarParts {
          mpn
          manufacturer { name }
          shortDescription
          category { name }
          specs {
            attribute { shortname }
            value
          }
          medianPrice1000 {
            convertedPrice
            convertedCurrency
          }
          estimatedFactoryLeadDays
          sellers {
            company { name }
            offers {
              inventoryLevel
              moq
              prices { quantity, price, currency }
            }
          }
        }
      }
    }
  }
}
```

**How Octopart determines similarity:**
- Not publicly documented in detail
- Based on Octopart's component taxonomy and specification matching
- Parts are matched within the same category with similar key parameters
- Likely uses a combination of: same category, similar package, similar key electrical specs
- Does NOT guarantee pin-compatibility -- the agent must verify this

**Pricing tier constraints:**
- **Evaluation (free):** 100 lifetime matched parts. `similarParts` IS included.
- **Standard:** 2,000 parts/month. `similarParts` is NOT included.
- **Pro:** 15,000 parts/month. `similarParts` is NOT included (listed as "Additional" option).
- **Enterprise:** Custom. `similarParts` IS included.

This is a significant constraint: the `similarParts` field is only available on the free Evaluation tier (100 lifetime parts -- useless for real work) and the Enterprise tier (custom pricing, likely expensive). The Standard and Pro tiers explicitly exclude it.

**Implication:** For production use of `similarParts`, you need either Enterprise pricing from Nexar, or an alternative approach.

---

### 2.2 Mouser API -- `SuggestedReplacement` Field

**Status:** Already integrated in datasheet-cli. Field is fetched but not displayed.

**Source:** `src/mouser.rs` -- the `Part` struct already deserializes `SuggestedReplacement`.

**What it provides:**
- A single part number string suggesting a replacement, typically populated for EOL/NRND parts
- Mouser's own recommendation for what to use instead
- Limited to one suggestion per part
- Only populated when Mouser has flagged the part as needing replacement

**Limitations:**
- Only one alternative per part (not a list)
- Only populated for parts that Mouser considers EOL/NRND
- Does not provide similarity scoring or parametric comparison
- Mouser-specific (only suggests parts Mouser stocks)

**Value:** Low effort to surface (already fetched). Useful as a "last resort" signal that a part is being phased out and Mouser has a suggestion. Not useful for proactive alternative finding.

**No dedicated cross-reference endpoint exists in the Mouser API (V1 or V2).**

---

### 2.3 DigiKey API -- Product Information V4

**Status:** Already integrated in datasheet-cli.

**Cross-reference capability:** The DigiKey Product Information V4 API does not have a dedicated cross-reference or "similar parts" endpoint based on available documentation. The API includes:

- `POST /products/v4/search/keyword` -- keyword search with filters
- `GET /products/v4/search/{partNumber}/productdetails` -- detailed part info
- `GET /products/v4/search/{productNumber}/pricing` -- pricing details
- `GET /products/v4/search/{productNumber}/pricingbyquantity/{qty}` -- quantity-based pricing

The `ProductDetails` response includes `PartStatus` (Active, Obsolete, Discontinued, etc.) which is lifecycle data, but no `SubstituteParts` or `AssociatedProducts` field has been confirmed in the API response schema.

**Parametric search workaround:** DigiKey's keyword search supports filtering by category and parametric values (package type, voltage, current, etc.). An agent could:
1. Get the category and key specs of the original part
2. Search DigiKey with the same category + parametric filters
3. Filter results to find parts with matching specs

This is essentially a parametric search approach (level 3 compatibility), not a true cross-reference.

---

### 2.4 JLCPCB/LCSC -- Category-Based Search

**Status:** Already integrated in datasheet-cli.

**Cross-reference capability:** No dedicated cross-reference API. However, the JLCPCB parts catalog is organized by category with parametric filters:

- **Category hierarchy:** e.g., Amplifiers > Operational Amplifiers > Precision Op Amps
- **Package filter:** SOT-23, DIP-8, SOIC-8, etc.
- **Manufacturer filter**
- **Basic/Extended part classification**

**Workaround approach:** Given a part's category and package:
1. Search JLCPCB for the same category
2. Filter by package type
3. Compare specs from the results

**LCSC official API** (documented at `https://www.lcsc.com/docs/openapi/index.html`) provides:
- `GET /rest/wmsc2agent/category/product/{category_id}` -- products by category with filters
- Better structured than the undocumented JLCPCB cart API

This approach is particularly valuable for JLCPCB assembly optimization: finding "basic" or "preferred" category alternatives to "extended" parts (which incur setup fees).

---

### 2.5 TME (Transfer Multisort Elektronik) -- `GetSimilarProducts`

**Status:** Not yet integrated in datasheet-cli. Documented in `stock-pricing-apis.md`.

**Cross-reference capability:** TME has a dedicated `GetSimilarProducts` endpoint.

**API details:**
- **Endpoint:** `POST /Products/GetSimilarProducts.json`
- **Parameters:** Product symbol(s), up to 50 per request
- **Auth:** HMAC-SHA1 token signature
- **Rate limit:** 10 req/sec (general), 2 req/sec (pricing/stock)

**What it returns:** A list of TME product symbols that TME considers similar to the input part. The similarity criteria are TME's internal classification (not publicly documented).

**Limitations:**
- TME-centric: only returns parts TME stocks
- European distributor -- inventory skews European
- No documented similarity scoring or compatibility level
- Need to make follow-up calls to get specs/pricing for the similar parts

**Value:** Medium. Useful as an additional data point, especially for European-sourced designs. The batch capability (50 parts per request) is nice.

---

### 2.6 Manufacturer Cross-Reference Tools

Many semiconductor manufacturers offer cross-reference tools on their websites that map competitor part numbers to their own equivalents:

**Texas Instruments:** `ti.com/cross-reference` -- maps competitor parts to TI equivalents. Web-only, no public API. Covers analog, power, and logic ICs.

**onsemi:** `onsemi.com/cross-reference` -- maps competitor parts to onsemi equivalents. Web-only.

**Nexperia:** Cross-reference tool for discretes and logic. Web-only.

**STMicroelectronics:** MCU cross-reference tool. Web-only.

**Microchip:** Has a "Competitor Cross-Reference" tool. Web-only.

**Pattern:** These are all web-based tools without public APIs. They are biased toward the manufacturer's own products (obviously). They could theoretically be scraped, but this is fragile and likely against ToS.

**Value for datasheet-cli:** Low. These are useful for manual engineering work but not practical for automated cross-reference. The data they encode (which competitor parts map to which of their parts) is essentially what Octopart/Nexar's `similarParts` aggregates across all manufacturers.

---

### 2.7 FindChips / SupplyFrame

**Overview:** FindChips (owned by SupplyFrame, which is owned by Siemens) is an aggregator similar to Octopart.

**Capabilities:**
- Distributor search with real-time pricing/stock
- Parametric search by category
- Part comparison tools
- Access to CAD models, footprints, reference designs

**API:** No public API documented. FindChips is primarily a web tool.

**Note:** parts.io redirects to FindChips, so they appear to have been merged.

**Value:** Not useful for programmatic access. Nexar/Octopart is the better option for API-based aggregation.

---

### 2.8 Component Databases and CAD Libraries

**SnapEDA/SnapMagic:** Already integrated in datasheet-cli. Provides CAD symbols and footprints but no cross-reference or similar parts feature.

**SamacSys / Component Search Engine:** Provides CAD models. No public cross-reference API found.

**Ultra Librarian:** CAD model library. No public cross-reference API found.

**AllDatasheet:** Large datasheet repository. Has a web-based cross-reference for some part families (especially transistors and discrete semiconductors). No public API.

---

### 2.9 EDA Tool Part Libraries

**Altium 365 / Octopart integration:** Altium Designer has built-in Octopart integration that shows similar parts. This is the same data as Nexar's `similarParts` but accessed through the Altium UI rather than API.

**KiCad component libraries:** Community-maintained, no cross-reference metadata.

---

## 3. LLM-Based Cross-Reference (The Pragmatic Approach)

Given the limitations of available APIs (Nexar's `similarParts` is paywalled behind Enterprise, manufacturer tools lack APIs, distributor APIs lack cross-reference), the most practical approach for an LLM agent may be **LLM-assisted parametric search**:

### How it works:

1. **Extract key specs** from the original part (already done via `datasheet extract characteristics`):
   - Category (e.g., "LDO voltage regulator")
   - Key electrical params (Vin range, Vout, Iout, dropout voltage)
   - Package (e.g., SOT-223, SOT-23-5)
   - Special features (enable pin, soft-start, thermal shutdown)

2. **Construct search queries** using the LLM's knowledge:
   - The LLM already knows common cross-references (AMS1117 ~ LM1117 ~ AP1117)
   - The LLM can generate parametric search terms from specs
   - For common parts, the LLM can directly suggest well-known alternatives

3. **Search distributors** with those queries:
   - `datasheet mouser search "3.3V LDO SOT-223 1A"`
   - `datasheet digikey search "LDO 3.3V 1A fixed output"`
   - `datasheet jlcpcb search "AMS1117-3.3"`

4. **Compare results** against original specs:
   - Extract characteristics of candidate parts
   - LLM compares specs and flags differences
   - LLM identifies pin-compatibility from pinout data

This approach leverages the LLM's broad knowledge of electronics components and the existing distributor search APIs, without requiring a dedicated cross-reference API.

**Advantages:**
- Works with existing datasheet-cli infrastructure
- No additional API keys or costs
- LLMs are surprisingly good at knowing common cross-references
- Can handle all levels of compatibility (pin-compatible through application-equivalent)

**Disadvantages:**
- LLM knowledge has a training cutoff -- may miss very new parts
- Not systematic -- may miss less-known alternatives
- Requires multiple API calls per cross-reference check
- LLM may hallucinate part numbers that don't exist

---

## 4. Proposed CLI Interface

### 4.1 Primary Command: `datasheet cross-ref`

```bash
# Find alternatives for a part (uses Nexar if available, falls back to parametric search)
datasheet cross-ref <part-number> [--json]

# Find alternatives with specific constraints
datasheet cross-ref <part-number> --package SOT-223 [--json]     # must match package
datasheet cross-ref <part-number> --pin-compatible [--json]       # only pin-compatible
datasheet cross-ref <part-number> --jlcpcb-basic [--json]         # prefer JLCPCB basic parts
datasheet cross-ref <part-number> --in-stock [--json]             # only parts currently in stock
datasheet cross-ref <part-number> --max-price 2.00 [--json]       # price ceiling

# Check lifecycle status
datasheet lifecycle <part-number> [--json]
```

### 4.2 Output Format

Human-readable:
```
Cross-reference for AMS1117-3.3 (AMS / Advanced Monolithic Systems)
  Category: LDO Voltage Regulator, 3.3V Fixed, 1A
  Package: SOT-223
  Status: Active

  Pin-Compatible Alternatives (same SOT-223 pinout):
    1. LM1117IMP-3.3/NOPB (Texas Instruments)
       Status: Active | Mouser: 15,234 in stock | $0.72 @100
    2. AP1117E33G-13 (Diodes Inc)
       Status: Active | Mouser: 8,901 in stock | $0.38 @100
    3. NCP1117ST33T3G (onsemi)
       Status: Active | DigiKey: 12,456 in stock | $0.45 @100

  Functionally Equivalent (different package/pinout):
    4. AP2112K-3.3TRG1 (Diodes Inc) -- SOT-23-5
       Status: Active | JLCPCB Basic | $0.15 @100
    5. MIC5219-3.3YM5-TR (Microchip) -- SOT-23-5
       Status: Active | Mouser: 45,678 in stock | $0.52 @100
```

JSON output:
```json
{
  "original": {
    "mpn": "AMS1117-3.3",
    "manufacturer": "AMS",
    "category": "LDO Voltage Regulator",
    "package": "SOT-223",
    "key_specs": {
      "output_voltage": "3.3V",
      "max_output_current": "1A",
      "dropout_voltage": "1.3V",
      "input_voltage_max": "15V"
    },
    "lifecycle_status": "Active"
  },
  "alternatives": [
    {
      "mpn": "LM1117IMP-3.3/NOPB",
      "manufacturer": "Texas Instruments",
      "compatibility_level": "pin_compatible",
      "package": "SOT-223",
      "key_specs": { ... },
      "lifecycle_status": "Active",
      "availability": {
        "mouser": { "stock": 15234, "price_100": 0.72 },
        "digikey": { "stock": 22100, "price_100": 0.68 }
      }
    },
    ...
  ]
}
```

### 4.3 Lifecycle Status Command

```bash
datasheet lifecycle <part-number> [--json]
```

Aggregates lifecycle data from multiple sources:
- Mouser: `LifecycleStatus` field (already fetched)
- DigiKey: `PartStatus` field (already fetched), plus `IsDiscontinued`, `IsObsolete`, `IsEndOfLife` from pricing endpoint
- Nexar: lifecycle status in `specs` (attribute shortname `lifecyclestatus`)
- JLCPCB: no lifecycle data available

```
Lifecycle Status for STM32F407VET6:
  Mouser:  Active
  DigiKey: Active
  Nexar:   Production
  Overall: ACTIVE -- safe to design in
```

---

## 5. How an LLM Agent Would Use This in Practice

### During Phase 1 (Part Selection):

```
Agent workflow for selecting a voltage regulator:

1. Agent identifies need: "3.3V LDO, >500mA, SOT-223 preferred"
2. Agent searches: `datasheet mouser search "3.3V LDO 1A SOT-223" --json`
3. Agent picks AMS1117-3.3 as primary candidate
4. Agent checks availability: `datasheet mouser part AMS1117-3.3 --json`
   -> Sees: In stock, Active, $0.35 @100
5. Agent checks cross-references: `datasheet cross-ref AMS1117-3.3 --json`
   -> Gets list of pin-compatible and functional alternatives
6. Agent records in decision note:
   - Primary: AMS1117-3.3 (cheapest, widely available)
   - Alt 1: LM1117IMP-3.3 (pin-compatible, TI brand, slightly more expensive)
   - Alt 2: AP2112K-3.3 (different package, JLCPCB basic, cheapest for JLCPCB assembly)
7. Agent proceeds with AMS1117-3.3 for schematic, notes alternatives
```

### When a Part Becomes Unavailable:

```
Agent workflow when primary part is out of stock:

1. During availability check: `datasheet check-availability STM32F407VET6 --json`
   -> All distributors: 0 stock, lead time 52 weeks
2. Agent runs: `datasheet cross-ref STM32F407VET6 --pin-compatible --in-stock --json`
   -> Returns pin-compatible STM32F4 variants that are in stock
3. Agent evaluates alternatives:
   - Same family: STM32F407VGT6 (more flash), STM32F405VGT6 (slightly different peripherals)
   - Different family: STM32F446VET6 (newer, faster, pin-compatible)
4. Agent extracts datasheets for top candidates
5. Agent compares key specs (flash, RAM, peripherals, clock speed)
6. Agent recommends switch and documents rationale
```

### For BOM Optimization:

```
Agent workflow for optimizing a BOM for JLCPCB assembly:

1. Agent has a complete BOM with 25 unique parts
2. Agent runs: `datasheet bom-check bom.json --json`
   -> Identifies 5 parts that are JLCPCB "extended" (extra fee)
3. For each extended part, agent runs:
   `datasheet cross-ref <part> --jlcpcb-basic --json`
   -> Finds basic/preferred alternatives for 3 of 5 parts
4. Agent evaluates whether the alternatives meet design requirements
5. Agent updates BOM and saves $15-30 in assembly fees
```

---

## 6. Recommended Implementation Approach

### Phase 1: Surface Existing Cross-Reference Data (Low Effort)

**What:** Add a `cross-ref` subcommand that uses data already being fetched.

- Display Mouser's `SuggestedReplacement` field when populated
- Query lifecycle status from Mouser (`LifecycleStatus`) and DigiKey (`PartStatus`)
- Combine into a single `datasheet lifecycle <part-number>` command

**Implementation:** ~1-2 days. No new API integrations. Just new CLI subcommands that format existing data.

### Phase 2: Parametric Search Cross-Reference (Medium Effort)

**What:** Implement category-based alternative finding using existing distributor APIs.

1. Look up the original part to get its category and key specs
2. Search the same category on Mouser/DigiKey/JLCPCB with spec filters
3. Filter and rank results by spec similarity
4. Present as a cross-reference list

**Implementation:** ~3-5 days. Uses existing search APIs. The main work is:
- Mapping Mouser/DigiKey/JLCPCB categories to each other
- Defining "key specs" per category (voltage for regulators, capacitance for caps, etc.)
- Ranking algorithm for similarity

**Challenge:** Category mapping across distributors is non-trivial. Each distributor has its own taxonomy. Octopart/Nexar solves this with their unified taxonomy, but that requires Nexar integration.

### Phase 3: Nexar `similarParts` Integration (Medium-High Effort)

**What:** Integrate Nexar API with `similarParts` support.

**Prerequisite:** Nexar integration (already planned in `stock-pricing-apis.md` Phase 3).

**Implementation:**
1. Add `similarParts` to the Nexar GraphQL query
2. Add `datasheet nexar cross-ref <mpn> --json` subcommand
3. Parse and display similar parts with their specs, pricing, and availability

**Blocker:** `similarParts` is only available on Enterprise tier. Need to evaluate pricing. If Enterprise is too expensive, fall back to parametric search approach.

**Alternative:** Use the Nexar free Evaluation tier (100 parts) for development and testing, then decide whether Enterprise pricing is justified for production use.

### Phase 4: LLM-Augmented Cross-Reference (Higher Effort, Highest Value)

**What:** Use the LLM (already available via `datasheet extract`) to intelligently find alternatives.

1. Extract characteristics of the original part
2. Ask the LLM to suggest known alternatives based on its training data
3. Verify LLM suggestions by searching distributors
4. Present verified alternatives with real availability data

**Implementation:**
```bash
datasheet cross-ref <part-number> --smart [--json]
```

This would:
1. Run `datasheet extract characteristics <datasheet.pdf>` (if datasheet available)
2. Send a prompt to the LLM: "Given this part with these specs, suggest pin-compatible and functionally equivalent alternatives"
3. For each LLM suggestion, verify it exists: `datasheet mouser part <suggestion> --json`
4. For verified parts, extract and compare specs
5. Present ranked results

**Advantage:** Leverages LLM's broad component knowledge. Can handle cross-manufacturer cross-references that no single API provides.

**Cost:** Each cross-reference check would cost 1 LLM API call (Gemini, via existing extract infrastructure) + N distributor API calls for verification.

### Summary of Phases

| Phase | Feature | Effort | Value | Dependencies |
|-------|---------|--------|-------|-------------|
| 1 | Surface existing data (lifecycle, SuggestedReplacement) | Low | Medium | None |
| 2 | Parametric search cross-reference | Medium | High | None |
| 3 | Nexar `similarParts` | Medium | High | Nexar integration, Enterprise tier |
| 4 | LLM-augmented cross-reference | High | Highest | Gemini API key (already required) |

**Recommended order:** Phase 1 -> Phase 2 -> Phase 4 -> Phase 3 (only if Enterprise pricing is reasonable).

Phase 4 (LLM-augmented) is ranked before Phase 3 (Nexar) because:
- It uses infrastructure already in place (Gemini LLM, distributor APIs)
- No additional API costs beyond existing Gemini usage
- More flexible than Nexar's opaque similarity algorithm
- Not gated behind Enterprise pricing

---

## 7. Open Questions

1. **Nexar Enterprise pricing:** What does the Enterprise tier cost? Is `similarParts` available as an add-on to Pro? Need to contact Nexar sales.

2. **Mouser `SuggestedReplacement` coverage:** How often is this field populated? Is it only for EOL parts, or does Mouser suggest replacements more broadly? Need to test with a range of parts.

3. **DigiKey substitutes data:** Some forum posts suggest DigiKey's product detail response may include `AssociatedProducts` or `SubstituteProducts` in some cases. Need to test the actual API response for EOL parts to see if this field exists and is populated.

4. **JLCPCB category taxonomy:** Is there a stable mapping between JLCPCB component categories and standard categories (e.g., IPC component classification)? This would enable cross-distributor category matching.

5. **LLM cross-reference accuracy:** How reliable are LLM suggestions for pin-compatible alternatives? Need to evaluate against known cross-reference pairs (e.g., LM1117 family). Risk of hallucinated part numbers that don't exist.

6. **Pin-compatibility verification:** Even when a cross-reference source says parts are "similar," pin compatibility must be verified. Should the agent automatically extract and compare pinouts of the original and alternative parts? This would require 2x datasheet downloads and extractions per alternative.

7. **TME `GetSimilarProducts`:** What similarity criteria does TME use? Is the response useful for cross-manufacturer cross-reference, or does it only suggest other TME-stocked variants? Need to test with actual API access.
