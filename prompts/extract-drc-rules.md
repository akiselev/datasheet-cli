**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior PCB Design Engineer and Altium Constraint Manager.

**Objective:** Analyze the attached datasheet and extract specific design constraints, mapping them directly to the provided `RuleKind` enum list.

**Context:** Your output will be parsed by a Rust-based tool to generate specific PCB design rules. You must strictly use the Enum variants provided below as keys.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO design rules or layout guidelines exist, respond with: `{"error": "No design rules found", "part_number": "...", "pages_searched": [...]}`

---

## THE RULEKIND ENUM (USE THESE EXACT NAMES)

| RuleKind | Description | Example Usage |
|----------|-------------|---------------|
| `Clearance` | Spacing between objects (high voltage, creepage) | "Maintain 0.5mm clearance between VIN and GND" |
| `Width` | Trace width constraints (current carrying, impedance) | "Power traces minimum 0.5mm wide" |
| `RoutingVias` | Specific via types/sizes (thermal vias) | "Use 0.3mm vias for thermal pad" |
| `PlaneConnect` | Connection style to planes (thermal relief vs direct) | "Use direct connect for power pins" |
| `PolygonConnect` | Polygon pour connection style | "Connect thermal pad directly to ground polygon" |
| `SolderMaskExpansion` | Mask expansion/swelling (SMD vs NSMD) | "Use NSMD with 0.05mm mask opening reduction" |
| `PasteMaskExpansion` | Stencil apertures for paste | "Use 88% paste coverage on thermal pad" |
| `Height` | Maximum component height (mechanical) | "Maximum component height 3mm" |
| `DiffPairsRouting` | Differential pair gaps/widths | "USB D+/D- 90 ohm differential" |
| `MaxMinImpedance` | Characteristic impedance requirements | "50 ohm single-ended for RF signals" |
| `Length` | Trace length limits | "Maximum 50mm for clock traces" |
| `NetAntennae` | Stub limits (high frequency) | "No stubs longer than 2mm on high-speed signals" |
| `FanoutControl` | BGA/LCC breakout strategies | "Use dog-bone fanout for BGA" |
| `ComponentClearance` | 3D spacing requirements | "Keep 1mm clearance around inductor" |

---

## EXTRACTION INSTRUCTIONS

### Step 1: Search for Design Constraints
Look in these sections:
- "Layout Guidelines"
- "PCB Layout Recommendations"
- "Thermal Information"
- "Application Information"
- "Mechanical Data"
- "Package Information"

### Step 2: For EACH Constraint Found, Extract:

| Field | Requirement |
|-------|-------------|
| `rule_kind` | EXACT RuleKind enum name from table above |
| `target_type` | "NetClass", "Net", "Component", "Pin", or "Global" |
| `target_names` | Array of specific nets/pins this applies to |
| `value` | Numeric value with unit (e.g., "Min 0.5mm") |
| `condition` | When this applies (e.g., "for currents > 1A") |
| `source_text` | EXACT quote from datasheet |
| `source_page` | 0-indexed page number |

### Step 3: Map Constraints to RuleKind
Follow these mapping guidelines:

**Trace/Routing Constraints:**
- "Use wide traces for power" → `Width`
- "Minimize trace length" → `Length`
- "90 ohm differential impedance" → `DiffPairsRouting` or `MaxMinImpedance`

**Thermal Constraints:**
- "Connect thermal pad to ground plane" → `PolygonConnect`
- "Use thermal vias under pad" → `RoutingVias`
- "Direct connection to power plane" → `PlaneConnect`

**Manufacturing Constraints:**
- "NSMD preferred" → `SolderMaskExpansion`
- "Reduce paste aperture by 10%" → `PasteMaskExpansion`

**Spacing Constraints:**
- "High voltage creepage 2mm" → `Clearance`
- "Keep 3mm from inductor" → `ComponentClearance`

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List rules grouped by RuleKind, then alphabetically within each group
2. **Completeness:** Extract ALL constraints mentioned, not just common ones
3. **Exactness:** Quote source text exactly (preserve wording)
4. **Enum Compliance:** ONLY use RuleKind values from the table above
5. **Units:** Always include units (mm, mil, ohm, %, etc.)

---

## IF DATA NOT FOUND

- If no layout guidelines exist: Return error response (see above)
- If a constraint doesn't map to any RuleKind: Omit it (do not invent new RuleKind values)
- If target nets are not specified: Use `"target_type": "Global"`
- If condition is not specified: Set `"condition": null`

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [8, 15, 22],
  "design_rules": [
    {
      "rule_kind": "Width",
      "applicability": {
        "target_type": "Net",
        "target_names": ["VIN", "VOUT", "SW"]
      },
      "constraint_details": {
        "value": "Min 0.5mm (20mil)",
        "condition": "For currents up to 2A"
      },
      "source_text": "Use wide traces (minimum 0.5mm) for power connections to minimize voltage drop.",
      "source_page": 15
    },
    {
      "rule_kind": "PolygonConnect",
      "applicability": {
        "target_type": "Pin",
        "target_names": ["Thermal Pad", "EP"]
      },
      "constraint_details": {
        "value": "Direct connect (no thermal relief)",
        "condition": null
      },
      "source_text": "Connect the exposed thermal pad directly to the ground plane without thermal relief.",
      "source_page": 15
    },
    {
      "rule_kind": "RoutingVias",
      "applicability": {
        "target_type": "Pin",
        "target_names": ["Thermal Pad"]
      },
      "constraint_details": {
        "value": "9x 0.3mm vias in 3x3 array",
        "condition": "Under thermal pad"
      },
      "source_text": "Place an array of thermal vias (0.3mm diameter, 3x3 grid) under the exposed pad.",
      "source_page": 16
    },
    {
      "rule_kind": "DiffPairsRouting",
      "applicability": {
        "target_type": "NetClass",
        "target_names": ["USB"]
      },
      "constraint_details": {
        "value": "90 ohm differential, gap=trace width",
        "condition": null
      },
      "source_text": "Route USB D+ and D- as 90 ohm differential pair with equal gap and trace width.",
      "source_page": 22
    },
    {
      "rule_kind": "Clearance",
      "applicability": {
        "target_type": "Net",
        "target_names": ["VIN"]
      },
      "constraint_details": {
        "value": "Min 2mm to low-voltage signals",
        "condition": "When VIN > 30V"
      },
      "source_text": "Maintain minimum 2mm creepage distance between high-voltage input and low-voltage signals.",
      "source_page": 8
    },
    {
      "rule_kind": "PasteMaskExpansion",
      "applicability": {
        "target_type": "Pin",
        "target_names": ["Thermal Pad"]
      },
      "constraint_details": {
        "value": "60% coverage, window pane pattern",
        "condition": null
      },
      "source_text": "Use a window-pane stencil pattern providing approximately 60% paste coverage on the thermal pad.",
      "source_page": 17
    }
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL layout constraints from the document are extracted
- [ ] `rule_kind` values are ONLY from the RuleKind enum table
- [ ] `source_text` is quoted exactly from the document
- [ ] Values include units
- [ ] Target nets/pins are identified correctly
- [ ] Source page numbers are 0-indexed and accurate
