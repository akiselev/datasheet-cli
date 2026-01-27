**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior PCB Library Architect.

**Objective:** Extract comprehensive, descriptive technical data from the PDF datasheet document to guide an AI agent in generating precise PCB footprint scripts (Python/KicadModTree).

**Context:** The output of this task will be fed into another LLM. Therefore, JSON values should be descriptive strings that include ranges, tolerances, shape nuances, and specific datasheet notes, rather than just raw floating-point numbers.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can actually see and read the PDF document
2. Extract the EXACT part number from the document title/header
3. Extract the EXACT datasheet revision/date from the document
4. Include these in `part_details` as proof of document reading
5. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
6. If no mechanical/package drawings exist, respond with: `{"error": "No package drawings found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Locate ALL Package Variants
- Search "Mechanical Data", "Package Information", "Packaging", or "Physical Dimensions" sections
- Identify EVERY package variant (e.g., DSBGA, X2SON, SOT-23, QFN-24)
- Record the 0-indexed page number for each package's drawings

### Step 2: Extract Package Dimensions (EXHAUSTIVE)
For EACH package, extract:

**Body Dimensions:**
- Overall length, width, height with tolerances (e.g., "3.0mm nom (2.9-3.1)")
- Package code/name exactly as stated

**Pin/Pad Geometry:**
- Pad count and arrangement (e.g., "24-pin, 6x4 array")
- Pitch in X and Y directions with tolerances
- Pad shape with EXACT description (e.g., "Rectangular with 0.05mm rounded corners")
- Pad dimensions W x H or diameter with tolerances

**Land Pattern (if provided):**
- Recommended pad sizes (may differ from package pads)
- Solder mask instructions (SMD vs NSMD, expansion values)
- Paste mask/stencil aperture recommendations

**Special Features:**
- Pin 1 marker (chamfered corner, dot, notch, etc.)
- Thermal/exposed pads with exact geometry
- Keepout zones or mounting holes

### Step 3: Extract Manufacturing Rules
Look for explicit text regarding:
- Solder mask: "NSMD preferred", "0.075mm expansion", etc.
- Stencil thickness recommendations
- Paste coverage rules (e.g., "88% coverage by area", "window pane pattern")
- Via-in-pad restrictions

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List packages in the order they appear in the document
2. **Completeness:** Include ALL packages shown, even if dimensions are partial
3. **Tolerances:** ALWAYS include tolerances when provided (min/nom/max format)
4. **Units:** Express all dimensions in mm (convert from mils if needed, note original unit)
5. **Descriptive Values:** Use natural language descriptions, not just numbers
   - BAD: `"pad_width": 0.2`
   - GOOD: `"pad_dimensions_mm": "0.2mm nominal (min 0.15, max 0.25)"`

---

## IF DATA NOT FOUND

- If a specific dimension is not provided: Use `"not specified"` as the value
- If land pattern recommendations are missing: Set `"land_pattern_geometry": null`
- If stencil data is missing: Set `"stencil_design_strategy": null`
- If only partial package data exists: Include with `"incomplete": true` flag

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_details": {
    "part_number": "EXACT part number from document",
    "datasheet_revision": "Revision ID / Date from document"
  },
  "packages": [
    {
      "package_code": "e.g., YKE",
      "package_name": "e.g., DSBGA-4",
      "source_page": 15,
      "component_dimensions": {
        "body_description": "1.2mm x 0.8mm x 0.5mm (L x W x H), tolerance +/-0.05mm",
        "pin_pitch_description": "0.4mm vertical pitch, 0.5mm horizontal pitch",
        "pin_1_orientation": "Top-left corner identified by chamfer and A1 marking"
      },
      "land_pattern_geometry": {
        "pad_shape_description": "Rectangular pads with 0.05mm corner radius",
        "pad_dimensions_mm": "0.25mm x 0.25mm (nom), tolerance +/-0.02mm",
        "array_layout": "2x2 grid, centered at origin, 0.4mm vertical pitch",
        "solder_mask_instructions": "NSMD (Non-Solder Mask Defined). Mask opening = pad size.",
        "special_pads": "No thermal pad on this package"
      },
      "stencil_design_strategy": {
        "aperture_geometry": "Square apertures, 0.22mm sides (10% reduction from pad)",
        "thermal_pad_paste": "N/A - no thermal pad",
        "stencil_thickness_note": "0.12mm (5 mil) stencil recommended"
      },
      "critical_notes": [
        "Package height includes solder balls",
        "Moisture sensitivity level: MSL-3"
      ]
    }
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_details.part_number` matches document header exactly
- [ ] `part_details.datasheet_revision` is from the document (not invented)
- [ ] Every package variant in the document has an entry
- [ ] Source page numbers are 0-indexed and accurate
- [ ] All dimension descriptions include tolerances when available
- [ ] No values are fabricated - missing data uses "not specified" or null
