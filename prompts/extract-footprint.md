**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior PCB Library Architect with deep knowledge of IPC-7351B land pattern standards.

**Objective:** Extract precise, machine-readable PCB footprint data from the PDF datasheet — including exact pad coordinates, dimensions, and thermal pad geometry — suitable for directly generating PCB footprint files without manual interpretation.

**Context:** The output will be consumed by an automated footprint generator. Every pad must have exact numerical coordinates and dimensions. When the datasheet provides a recommended land pattern, extract it. When only mechanical/package drawings are available, compute the land pattern using IPC-7351B nominal (Level B) guidelines — and mark it as computed.

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
- Search "Mechanical Data", "Package Information", "Packaging", "Physical Dimensions", "Land Pattern", "Recommended Footprint" sections
- Identify EVERY package variant (e.g., DSBGA, X2SON, SOT-23, QFN-24, SOIC-8)
- Record the 0-indexed page number for each package's drawings

### Step 2: Extract Mechanical Package Dimensions
For EACH package, extract the raw mechanical dimensions from the dimension table/drawing:

**Body Dimensions:**
- Overall length, width, height with min/max tolerances
- Package code/name exactly as stated

**Lead/Terminal Dimensions (for leaded packages):**
- Lead count and arrangement
- Lead pitch (e.g., 1.27mm)
- Lead width (min/max)
- Lead length/foot length (min/max)
- Lead span / tip-to-tip distance (min/max)
- Seating plane height

**Pad/Ball Dimensions (for BGA/LGA/QFN):**
- Pad/ball count and array arrangement
- Pitch in X and Y
- Pad/ball diameter or dimensions
- Exposed pad dimensions (for QFN/DFN)

### Step 3: Extract or Compute Land Pattern

**If the datasheet provides a recommended land pattern / suggested footprint:**
- Extract it directly — pad sizes, positions, mask openings
- Set `pad_data_source` to `"recommended_land_pattern"`

**If the datasheet only provides mechanical package dimensions (NO recommended land pattern):**
- Compute the land pattern using IPC-7351B Level B (nominal) guidelines
- Set `pad_data_source` to `"computed_ipc7351b"`
- Show your computation in `ipc7351b_computation`

**IPC-7351B computation rules for common package types:**

*Gull-wing leads (SOIC, SOP, SSOP, TSSOP, MSOP, QFP):*
- Toe extension (Jtoe) = 0.55mm, Heel extension (Jheel) = 0.45mm, Side extension (Jside) = 0.05mm (all Level B nominal)
- Zmax = Lmax (max lead span tip-to-tip)
- Gmin = Smin (min distance between opposite lead tips at inner edge = Lmin - 2 * max lead foot length, or if S is given directly, use it)
- Pad span outer edge = Zmax + 2 * Jtoe
- Pad span inner edge = Gmin - 2 * Jheel
- Pad length = ((Zmax + 2*Jtoe) - (Gmin - 2*Jheel)) / 2
- Pad center X = +/- ((Zmax + 2*Jtoe) + (Gmin - 2*Jheel)) / 4
- Pad width = max lead width + 2 * Jside
- Round all final values to 0.05mm grid

*QFN/DFN (bottom-terminated):*
- Toe extension = 0.40mm, Heel extension = 0.00mm, Side extension = 0.05mm
- Apply same span calculation using terminal dimensions
- Exposed thermal pad: use nominal dimensions from datasheet, or package body minus 0.2mm per side if not specified

*BGA/CSP:*
- Pad diameter = ball diameter (NSMD) or ball diameter minus 0.1mm
- Positions directly from pitch and array size

*SOT-23, SOT-223, SOT-89:*
- Use JEDEC/IPC-7351B standard footprint for the specific package

### Step 4: Build the Pads Array
Generate a `pads` array with one entry per pad. Coordinates use a standard origin at the geometric center of the footprint. Pin 1 is typically top-left.

**Coordinate convention:**
- Origin (0, 0) = geometric center of the component body
- X+ = right, Y+ = up (standard PCB convention)
- All dimensions in millimeters

For a dual-row package (e.g., SOIC-8 with pins 1-4 on left, 5-8 on right):
- Left column pads have negative X, right column pads have positive X
- Pads are spaced vertically at the pitch, centered around Y=0

### Step 5: Extract Manufacturing Rules
Look for explicit text regarding:
- Solder mask: "NSMD preferred", "0.075mm expansion", etc.
- Stencil thickness recommendations
- Paste coverage rules (e.g., "88% coverage by area", "window pane pattern")
- Via-in-pad restrictions

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List packages in the order they appear in the document
2. **Completeness:** Include ALL packages shown, even if dimensions are partial
3. **Tolerances:** ALWAYS include tolerances when provided (min/nom/max format) in the descriptive fields
4. **Units:** ALL numerical values in mm
5. **Precision:** Pad coordinates and dimensions to 0.01mm precision minimum
6. **Coordinate system:** Origin at component body center, X+ right, Y+ up
7. **Flat dimensions:** `component_dimensions` fields use flat keys like `body_length_min_mm`, `body_length_nom_mm`, `body_length_max_mm` — all plain numbers or `null`. NEVER nest objects inside `component_dimensions`.
8. **No fabricated manufacturing data:** Fields like `thermal_pad.via_array`, `thermal_pad.paste_pattern`, `thermal_pad.paste_coverage_percent`, `stencil_design_strategy` must be `null` unless the datasheet EXPLICITLY specifies them. Do NOT fill in generic defaults.

---

## IF DATA NOT FOUND

- If a specific mechanical dimension is not in the datasheet: Use `null` for that field
- If stencil data is missing: Set `"stencil_design_strategy": null`
- If only partial package data exists: Include with `"incomplete": true` flag
- NEVER leave pad coordinates or dimensions as "not specified" — either extract them from a recommended land pattern or compute them from mechanical dimensions

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
      "package_code": "e.g., DDA0008B",
      "package_name": "e.g., HSOP-8 / SOIC-8 with PowerPAD",
      "pin_count": 8,
      "source_page": 15,
      "pad_data_source": "recommended_land_pattern | computed_ipc7351b",
      "component_dimensions": {
        "body_length_min_mm": 4.8,
        "body_length_nom_mm": 4.9,
        "body_length_max_mm": 5.0,
        "body_width_min_mm": 3.8,
        "body_width_nom_mm": 3.9,
        "body_width_max_mm": 4.0,
        "body_height_max_mm": 1.7,
        "lead_span_min_mm": 5.8,
        "lead_span_max_mm": 6.2,
        "lead_pitch_mm": 1.27,
        "lead_width_min_mm": 0.31,
        "lead_width_max_mm": 0.51,
        "lead_length_min_mm": null,
        "lead_length_max_mm": null,
        "pin_1_indicator": "Dot on top-left corner"
      },
      "ipc7351b_computation": {
        "method": "gull_wing | qfn | bga | sot | chip | custom",
        "Zmax_mm": 6.2,
        "Gmin_mm": 3.8,
        "Jtoe_mm": 0.55,
        "Jheel_mm": 0.45,
        "Jside_mm": 0.05,
        "computed_pad_span_outer_mm": 7.3,
        "computed_pad_span_inner_mm": 2.9,
        "computed_pad_length_mm": 2.2,
        "computed_pad_width_mm": 0.61,
        "computed_pad_center_x_mm": 2.55,
        "notes": "Explain any assumptions or deviations from standard IPC-7351B"
      },
      "pads": [
        {
          "number": 1,
          "x_mm": -2.7,
          "y_mm": 1.905,
          "size_x_mm": 1.55,
          "size_y_mm": 0.6,
          "shape": "rectangle",
          "layers": "top_copper"
        },
        {
          "number": 2,
          "x_mm": -2.7,
          "y_mm": 0.635,
          "size_x_mm": 1.55,
          "size_y_mm": 0.6,
          "shape": "rectangle",
          "layers": "top_copper"
        }
      ],
      "thermal_pad": {
        "x_mm": 0.0,
        "y_mm": 0.0,
        "size_x_mm": 3.4,
        "size_y_mm": 2.8,
        "shape": "rectangle",
        "solder_mask_opening_x_mm": 2.71,
        "solder_mask_opening_y_mm": 3.4,
        "paste_coverage_percent": 50,
        "paste_pattern": "2x3 window pane, 0.2mm gap between sub-apertures",
        "via_array": "3x3 grid, 0.3mm drill, 0.6mm annular ring, filled and capped"
      },
      "solder_mask": {
        "type": "NSMD | SMD",
        "expansion_mm": 0.05
      },
      "courtyard": {
        "size_x_mm": 7.8,
        "size_y_mm": 5.4,
        "line_width_mm": 0.05
      },
      "stencil_design_strategy": {
        "aperture_geometry": "Match pad size or specify reduction",
        "thermal_pad_paste": "Window pane pattern details",
        "stencil_thickness_mm": 0.125
      },
      "critical_notes": [
        "Any important notes from the datasheet"
      ]
    }
  ]
}
```

**CRITICAL — `size_x_mm` and `size_y_mm` definition:**
- `size_x_mm` = pad extent in the **X direction** (horizontal on the PCB)
- `size_y_mm` = pad extent in the **Y direction** (vertical on the PCB)
- For a dual-row package (pins on left and right): pads are long in X (extending away from body) and narrow in Y (perpendicular to lead). So `size_x_mm` > `size_y_mm` for peripheral pads.
- **VALIDATION:** For pads arranged vertically at a given pitch, `size_y_mm` MUST be less than the pitch. For example, at 1.27mm pitch, `size_y_mm` must be < 1.27mm (typically 0.5-0.7mm). If size_y_mm >= pitch, you have swapped the dimensions — fix it.

**Notes on the pads array:**
- Include ALL pads (every signal pin). Do NOT omit pads or use shorthand like "pins 2-4 same as pin 1"
- The thermal/exposed pad should go in `thermal_pad`, NOT in the `pads` array
- For dual-row packages, number pads counterclockwise starting from pin 1 (top-left), matching the datasheet pinout
- `shape` is one of: `"rectangle"`, `"round"`, `"obround"`, `"rounded_rectangle"`
- `layers` is typically `"top_copper"` for SMD pads, `"through_hole"` for TH pads
- If `ipc7351b_computation` is null (because data came from a recommended land pattern), omit it

**Notes on the thermal_pad:**
- Only include if the package has an exposed thermal/ground pad
- If thermal pad dimensions are not explicitly stated but the package clearly has one, estimate from the mechanical drawing and note the estimation
- `paste_pattern`: describe the stencil aperture sub-division if specified
- `via_array`: describe recommended thermal vias if specified
- Set to `null` if no thermal pad

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_details.part_number` matches document header exactly
- [ ] `part_details.datasheet_revision` is from the document (not invented)
- [ ] Every package variant in the document has an entry
- [ ] Source page numbers are 0-indexed and accurate
- [ ] Every pad has exact `x_mm`, `y_mm`, `size_x_mm`, `size_y_mm` values (no nulls, no "not specified")
- [ ] For vertically-pitched pads, `size_y_mm` < pitch (pads must not overlap)
- [ ] Pad count in `pads` array matches `pin_count`
- [ ] Pad coordinates are symmetric and consistent with pitch
- [ ] `pad_data_source` correctly indicates whether data was extracted or computed
- [ ] If computed, `ipc7351b_computation` shows the method and intermediate values
