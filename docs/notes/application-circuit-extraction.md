# Research Note: Extracting Application Circuits as Structured Netlists

**Date:** 2026-03-17
**Author:** Research for datasheet-cli enhancement
**Status:** Proposal / research phase

---

## 1. Problem Statement

The `datasheet-cli` tool already has a `reference-design` extraction task that extracts a
BOM (bill of materials) from datasheet application circuits, including component values,
types, and basic two-pin connectivity descriptions. However, this extraction does **not**
capture the full circuit topology -- how components are connected to each other through
named nets. Without topology, an LLM agent building a `.schdoc-spec` schematic must
re-derive all connections from scratch, essentially re-reading the same datasheet images
that the extraction was supposed to automate.

**Goal:** Extract the "Typical Application Circuit" schematic as a structured netlist that
an LLM agent can directly translate into `.schdoc-spec` wiring (components, nets, wires,
power objects).

---

## 2. What Data Exists in Typical Application Circuits

Analysis of real datasheets in this project reveals several categories of information
present in application circuit diagrams:

### 2.1 Simple LDO/Regulator Circuits (e.g., AP2112, AMS1117)

- **Components:** 3-5 external parts (Cin, Cout, enable resistor)
- **IC shown as:** Rectangular box with labeled pins (VIN, VOUT, GND, EN)
- **Connections:** Simple: each passive connects between an IC pin and a power rail
- **Net names:** VIN, VOUT, GND are explicitly labeled; internal nodes often unnamed
- **Annotations:** Component values directly on the schematic (1uF, 10kohm)
- **Complexity:** Low -- almost always a linear chain of components

### 2.2 DC-DC Buck Converter Circuits (e.g., TPS5430, MP1584EN)

- **Components:** 7-12 external parts (Cin, Cout, L, D, Cboot, feedback divider R1/R2)
- **IC shown as:** Box with pin numbers and names (VIN, PH, BOOT, VSENSE, GND, ENA)
- **Connections:** More complex -- inductor-diode-capacitor loop, feedback divider chain
- **Net names:** VIN, VOUT, GND explicitly labeled; PH node connects inductor/diode/IC;
  VSENSE is the feedback divider midpoint
- **Annotations:** Component values, sometimes specific part numbers (B340A diode),
  voltage/current ratings
- **Complexity:** Medium -- includes a switching node with 3+ connections, feedback network

### 2.3 Motor Drivers / Complex ICs (e.g., DRV8871)

- **Components:** 3-5 external parts, but with external loads (motor, power supply)
- **IC shown as:** Box with pin names; external blocks shown as abstract symbols
- **Connections:** H-bridge outputs to motor, bypass + bulk caps, current-limit resistor
- **Net names:** VM, GND, OUT1, OUT2, ILIM explicitly labeled
- **Annotations:** Component values, design parameter tables alongside
- **Complexity:** Medium -- but includes off-board connections and abstract loads

### 2.4 Battery Charger Circuits (e.g., TP4056)

- **Components:** 5-10 external parts (LEDs, resistors, capacitors, NTC thermistor)
- **IC shown as:** Box with pin numbers and names
- **Connections:** Multiple application variants shown (with/without temp monitoring,
  with/without LED indicators, USB + wall adapter input)
- **Net names:** VCC, GND, BAT+, BAT-, partially labeled
- **Annotations:** Values on components, Chinese and English text mixed
- **Complexity:** Medium -- multiple circuit variants on the same page

### 2.5 Common Patterns Across All Datasheets

| Feature | Prevalence | Extraction Difficulty |
|---------|-----------|----------------------|
| IC pin names labeled on box | Universal | Easy (text) |
| Component values annotated | Universal | Easy (text/OCR) |
| Reference designators (C1, R1, L1) | ~80% | Easy (text/OCR) |
| Explicit net names (VIN, VOUT, GND) | ~70% | Medium (text + position) |
| Pin numbers on IC | ~60% | Medium (small text) |
| Junction dots at wire crossings | ~90% | Hard (small visual detail) |
| Multiple circuit variants per page | ~30% | Hard (which circuit to extract) |
| Component polarity indicators (+/-) | ~70% for polarized parts | Medium |
| Ground symbols | Universal | Easy (recognizable pattern) |
| Power rail symbols (VCC bars, arrows) | ~80% | Easy-Medium |

---

## 3. Existing Reference-Design Extraction: Gap Analysis

The current `extract-reference-design.md` prompt produces output like this (from TPS5430):

```json
{
  "required_components": [
    {
      "component_type": "Boot Capacitor",
      "designator": "C2",
      "recommended_value": "0.01uF",
      "connectivity": {
        "pin_1_connection": "BOOT",
        "pin_2_connection": "PH"
      }
    }
  ]
}
```

### What this captures:
- Component list with values, types, ratings
- Two-pin connectivity (pin_1 connects to X, pin_2 connects to Y)
- Placement notes (textual)

### What this misses (the gap):
- **No explicit net/node abstraction:** "BOOT" and "PH" are IC pin names, not net names.
  For a 3+ terminal component or a node where multiple components meet, this breaks down.
- **No topology graph:** There is no concept of "node N3 connects to: C2 pin 1, L1 pin 1,
  D1 cathode, and U1 pin 8 (PH)." Each component only knows about itself.
- **No multi-pin components:** Diodes, transistors, and ICs with >2 pins cannot be
  represented with just pin_1/pin_2.
- **No wire routing information:** Where do wires go? Which nodes join?
- **Ambiguous net naming:** "GND" is clear, but "PH" could be an IC pin name or a net
  name -- the distinction matters for schematic capture.
- **No ground/power symbol locations:** The schematic agent needs to know where to place
  power objects.

---

## 4. Prior Art: Automated Circuit-to-Netlist Systems

### 4.1 SINA (2026) -- Circuit Schematic Image-to-Netlist Generator

**Architecture:** Four-stage pipeline:
1. YOLOv11 detects components (bounding boxes + types)
2. Connected-Component Labeling (CCL) on the wire image (after masking components)
   segments wiring into distinct electrical nets
3. OCR extracts reference designators; intersection detection maps component terminals
   to net regions
4. GPT-4o generates final SPICE netlist from: OCR labels + component-to-node connectivity
   maps + original image for visual context

**Key insight:** The heavy lifting of connectivity inference is done by classical image
processing (CCL), not the LLM. The LLM's role is primarily to assign final reference
designators, component values, and produce the formatted netlist.

**Accuracy:** 96.47% overall netlist-generation accuracy (2.72x better than prior art).

**Relevance to our use case:** SINA requires a trained YOLO model for component detection,
which is impractical for a CLI tool operating on arbitrary datasheets. However, the insight
that LLMs are better at *interpreting* pre-processed connectivity data than at *inferring*
connectivity from raw images is directly applicable.

### 4.2 Auto-SPICE / Masala-CHAI (2024)

**Architecture:** Three-step workflow:
1. YOLOv8 detects components across 12 classes
2. Deep Hough Transform detects wires; line segments clustered into nets
3. Net annotations overlaid on the image in red; annotated image + structured prompt
   sent to GPT-4o for SPICE netlist generation

**Key insight on prompt engineering:** "By incorporating automatic net annotations into
the prompt, GPT-4o's performance in translating schematics to SPICE netlists improved
significantly." Providing the LLM with pre-computed net labels and component positions
reduced ambiguity dramatically compared to asking it to infer connectivity from raw images.

**Common LLM failure modes:**
- Confusing NMOS/PMOS transistor types
- Assuming intersecting lines always connect (missing junction dots)
- Confusing drain/source terminals on MOSFETs
- Omitting passive elements
- Incorrectly mapping differential pairs

**Post-extraction verification:** Python script checks for floating nets and structural
issues; LLM self-corrects errors in a feedback loop.

### 4.3 Netlistify (NVIDIA, 2025)

Uses YOLOv8 for component detection, ResNet for orientation, and a transformer model
for connectivity analysis. Trained on 100,000 synthetic schematic images. Won Best
Artifact Award at MLCAD'25.

**Relevance:** Demonstrates that connectivity extraction is the hardest sub-problem and
benefits most from deep learning approaches.

### 4.4 Key Takeaways from Prior Art

1. **Pure vision-LLM approaches (no preprocessing) have limited accuracy for connectivity.**
   All high-accuracy systems use classical CV or trained models for wire/net detection first.

2. **LLMs excel at the "last mile":** Given structured connectivity data, LLMs reliably
   produce formatted netlists with correct component values and designators.

3. **Net annotation on the image dramatically improves LLM accuracy.** When the LLM can
   see both the original schematic AND numbered net labels, it makes far fewer errors.

4. **SPICE netlist is the de facto output format** in all academic work, but our use case
   needs a different format optimized for schematic capture (not simulation).

---

## 5. Proposed Approach for datasheet-cli

Given the constraints of datasheet-cli (no trained YOLO models, no image preprocessing
pipeline, relies entirely on Gemini's multimodal capabilities for PDF analysis), we need
an approach that works within the LLM-only paradigm while acknowledging its limitations.

### 5.1 Strategy: Structured Netlist Extraction via Prompt Engineering

Rather than trying to replicate the full SINA/Auto-SPICE pipeline, we leverage the fact
that **datasheet application circuits are relatively simple** (typically 5-15 components)
compared to the full IC schematics those tools target. For these simple circuits, a
well-prompted multimodal LLM should achieve acceptable accuracy.

The approach:

1. **Use Gemini's native PDF vision** to analyze the application circuit pages
2. **Prompt for a structured netlist** using a carefully designed JSON schema
3. **Anchor everything to IC pin names** as the primary node identifiers
4. **Include verification steps** in the prompt (pin count checks, connectivity validation)
5. **Output a graph-based representation** (nodes + edges) rather than just a component list

### 5.2 Proposed Output JSON Schema

The schema is designed to be:
- Flat enough to stay within Gemini's schema nesting limits
- Rich enough to support direct translation to `.schdoc-spec`
- Self-validating (pin counts, net connectivity can be cross-checked)

```json
{
  "type": "object",
  "properties": {
    "part_number": { "type": "string" },
    "source_pages": {
      "type": "array",
      "items": { "type": "integer" }
    },
    "circuits": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "circuit_name": { "type": "string" },
          "circuit_type": { "type": "string" },
          "source_page": { "type": "integer" },
          "description": { "type": "string" },
          "design_parameters": {
            "type": "array",
            "items": { "type": "object" }
          },
          "components": {
            "type": "array",
            "items": { "type": "object" }
          },
          "nets": {
            "type": "array",
            "items": { "type": "object" }
          },
          "notes": {
            "type": "array",
            "items": { "type": "string" }
          }
        }
      }
    }
  },
  "required": ["part_number", "circuits"]
}
```

The key innovation over the existing `reference-design` schema is the **`nets` array**,
which explicitly models the circuit as a graph. Each net is a named electrical node that
lists all the component pins connected to it.

### 5.3 Detailed Output Format (Example)

For the TPS5430 12V-to-5V application circuit (Figure 7-1 in the datasheet):

```json
{
  "part_number": "TPS5430",
  "source_pages": [12, 13],
  "circuits": [
    {
      "circuit_name": "12V Input to 5.0V Output",
      "circuit_type": "typical",
      "source_page": 13,
      "description": "3A step-down converter, 10.8-19.8V input, 5V/3A output",
      "design_parameters": [
        { "parameter": "Input voltage range", "value": "10.8V to 19.8V" },
        { "parameter": "Output voltage", "value": "5V" },
        { "parameter": "Output current", "value": "3A" },
        { "parameter": "Switching frequency", "value": "500kHz" }
      ],
      "components": [
        {
          "designator": "U1",
          "type": "ic",
          "part_number": "TPS5430DDA",
          "description": "3A step-down converter IC",
          "pins": ["VIN", "ENA", "NC_2", "NC_3", "VSENSE", "GND", "BOOT", "PH", "DAP"]
        },
        {
          "designator": "C1",
          "type": "capacitor",
          "value": "10uF",
          "description": "Input decoupling capacitor",
          "voltage_rating": "25V",
          "dielectric": "X5R or X7R",
          "pins": ["1", "2"]
        },
        {
          "designator": "C2",
          "type": "capacitor",
          "value": "0.01uF",
          "description": "Bootstrap capacitor",
          "dielectric": "X5R or X7R",
          "pins": ["1", "2"]
        },
        {
          "designator": "C3",
          "type": "capacitor_polarized",
          "value": "220uF",
          "description": "Output filter capacitor",
          "voltage_rating": "10V",
          "dielectric": "POSCAP",
          "esr_max": "40mOhm",
          "pins": ["+", "-"]
        },
        {
          "designator": "L1",
          "type": "inductor",
          "value": "15uH",
          "description": "Output inductor",
          "current_rating": "3.4A",
          "pins": ["1", "2"]
        },
        {
          "designator": "D1",
          "type": "diode_schottky",
          "part_number": "B340A",
          "description": "Catch diode",
          "voltage_rating": "40V",
          "pins": ["A", "K"]
        },
        {
          "designator": "R1",
          "type": "resistor",
          "value": "10kohm",
          "description": "Feedback resistor (top)",
          "tolerance": "1%",
          "pins": ["1", "2"]
        },
        {
          "designator": "R2",
          "type": "resistor",
          "value": "3.24kohm",
          "description": "Feedback resistor (bottom)",
          "tolerance": "1%",
          "pins": ["1", "2"]
        }
      ],
      "nets": [
        {
          "name": "VIN",
          "type": "power_input",
          "voltage": "10.8-19.8V",
          "connections": [
            { "component": "U1", "pin": "VIN" },
            { "component": "C1", "pin": "1" }
          ]
        },
        {
          "name": "GND",
          "type": "ground",
          "connections": [
            { "component": "U1", "pin": "GND" },
            { "component": "U1", "pin": "DAP" },
            { "component": "C1", "pin": "2" },
            { "component": "C3", "pin": "-" },
            { "component": "D1", "pin": "A" },
            { "component": "R2", "pin": "2" }
          ]
        },
        {
          "name": "SW",
          "type": "internal",
          "description": "Switching node",
          "connections": [
            { "component": "U1", "pin": "PH" },
            { "component": "C2", "pin": "2" },
            { "component": "L1", "pin": "1" },
            { "component": "D1", "pin": "K" }
          ]
        },
        {
          "name": "BOOT",
          "type": "internal",
          "connections": [
            { "component": "U1", "pin": "BOOT" },
            { "component": "C2", "pin": "1" }
          ]
        },
        {
          "name": "VOUT",
          "type": "power_output",
          "voltage": "5V",
          "connections": [
            { "component": "L1", "pin": "2" },
            { "component": "C3", "pin": "+" },
            { "component": "R1", "pin": "1" }
          ]
        },
        {
          "name": "FB",
          "type": "internal",
          "description": "Feedback voltage divider midpoint",
          "connections": [
            { "component": "U1", "pin": "VSENSE" },
            { "component": "R1", "pin": "2" },
            { "component": "R2", "pin": "1" }
          ]
        },
        {
          "name": "EN",
          "type": "power_input",
          "description": "Enable input (active high, float to enable)",
          "connections": [
            { "component": "U1", "pin": "ENA" }
          ]
        }
      ],
      "notes": [
        "DAP (exposed pad) must be soldered to PCB ground for thermal performance",
        "Keep PH-L1-C3-GND loop as small as practical",
        "Place C1 as close to VIN pin as possible",
        "NC pins (2, 3) are not connected internally"
      ]
    }
  ]
}
```

### 5.4 Why This Format (Design Rationale)

**Graph-based (nets with connection lists) vs. edge-based (component-to-component):**

The nets-based representation was chosen because:
- It directly maps to how schematics work: nets are first-class objects
- It matches the `.schdoc-spec` DSL's `net` block syntax:
  ```
  net FB {
      pins: ["U1.VSENSE", "R1.2", "R2.1"]
  }
  ```
- It is self-validating: every component pin should appear in exactly one net
- It naturally handles multi-way junctions (3+ components sharing a node)
- Power nets (VIN, GND, VOUT) are explicitly typed, mapping to `power_object` in schdoc

**Component `pins` array:**

Each component lists its pin names. This serves dual purposes:
- Validates that every pin appears in some net (completeness check)
- Provides pin ordering that matches the schematic symbol for automated wiring

**Net `type` field:**

Classifying nets as `power_input`, `power_output`, `ground`, or `internal` directly
informs the schematic agent about which nets need power symbols vs. net labels vs.
explicit wires.

---

## 6. Prompt Engineering Considerations

### 6.1 Core Challenges for Gemini

Based on analysis of the datasheets and prior art failure modes, the key challenges are:

**Challenge 1: Junction Detection**
Wire crossings vs. connections. Schematics use a dot at intersections to indicate a
connection, and no dot to indicate wires crossing without connecting. This is a small
visual detail that LLMs frequently misinterpret. In datasheet application circuits, this
is less of a problem because most circuits are simple enough that wire crossings rarely
occur, but it still needs attention.

**Prompt mitigation:** Explicitly instruct the model: "Only consider wires connected at
a junction if there is a visible dot at the intersection. Wires that cross without a dot
are NOT electrically connected."

**Challenge 2: Implicit Ground Connections**
Many schematics show multiple ground symbols (the three-line triangle) scattered around
the circuit, all representing the same GND net. The model must understand that all ground
symbols merge into one net.

**Prompt mitigation:** "All ground symbols (downward-pointing triangle or three horizontal
lines) represent the same GND net. Merge all ground connections into a single net named
GND."

**Challenge 3: Component Pin Identity**
For two-pin passives (resistors, capacitors), pin identity (which end is "1" vs "2")
is often ambiguous in the schematic. For polarized components, polarity markers (+/-) are
critical.

**Prompt mitigation:** "For non-polarized two-pin components (resistors, non-polarized
capacitors), assign pin 1 to the terminal closest to the IC or to the higher-voltage net.
For polarized components (electrolytic capacitors, diodes), use +/- or A/K designations."

**Challenge 4: Multiple Circuits on One Page**
The TP4056 datasheet shows 5 different application circuits on a single page. The model
must distinguish between them and extract each separately.

**Prompt mitigation:** "If multiple application circuits appear, extract EACH as a separate
entry in the circuits array. Label each with its description from the datasheet text."

**Challenge 5: Unnamed Intermediate Nodes**
Internal nodes (like the switching node in a buck converter) often have no explicit label
in the schematic. The model must infer a reasonable name.

**Prompt mitigation:** "For nodes that are not explicitly labeled in the schematic, assign
a descriptive name based on function (e.g., 'SW' for a switching node, 'FB' for a
feedback node, 'BOOT' for a bootstrap node). Use the connected IC pin name as a fallback."

**Challenge 6: Off-Page Connections and Abstract Loads**
Application circuits often show abstract loads (a motor symbol, a battery symbol, "LOAD"
text) or off-page connections (arrows labeled "VIN", "VOUT"). These must be captured as
net endpoints without inventing phantom components.

**Prompt mitigation:** "For external connections shown as arrows or labels (VIN, VOUT),
create a net with that name. For abstract loads (motor, battery, load), create a component
entry with type 'external_load' or 'external_source' and note its nature."

### 6.2 Prompt Structure

Based on the patterns established by existing extraction prompts in the codebase, the
prompt should follow this structure:

1. **Anti-hallucination preamble** (same as other extraction tasks)
2. **Role and objective** -- "Act as a Circuit Analysis Engineer"
3. **Step-by-step extraction instructions:**
   - Step 1: Locate all application/reference circuits
   - Step 2: For each circuit, identify ALL components (IC + external)
   - Step 3: Trace every wire to build a connection graph
   - Step 4: Assign net names (explicit from schematic, inferred for unnamed nodes)
   - Step 5: Map every component pin to exactly one net
   - Step 6: Classify nets (power_input, power_output, ground, internal)
   - Step 7: Extract design parameters if present
4. **Consistency requirements** (exhaustive pin coverage, net naming conventions)
5. **Output schema with example**
6. **Verification checklist** (every pin in a net, no floating pins, etc.)

### 6.3 Verification Steps to Include in Prompt

The prompt should instruct the model to self-verify:

```
Before submitting, verify:
- [ ] Every component pin appears in exactly one net
- [ ] The IC's pin list matches the pinout from earlier in the datasheet
- [ ] Every net has at least 2 connections (no floating single-pin nets, except
      for external I/O nets like EN that may connect to off-board signals)
- [ ] GND net includes all ground symbols shown
- [ ] Power input/output nets are correctly typed
- [ ] Component count matches the schematic (count every distinct component visible)
- [ ] No component is listed twice
- [ ] Polarized components have correct polarity (+/- or A/K)
```

---

## 7. Challenges and Limitations

### 7.1 Accuracy Expectations

Based on prior art and the nature of the task:

- **Simple circuits (3-5 components, LDO type):** Expect 90-95% accuracy. These are
  nearly linear topologies where connectivity is obvious.
- **Medium circuits (7-12 components, buck converter type):** Expect 75-85% accuracy.
  The switching node junction and feedback divider chain add ambiguity.
- **Complex circuits (15+ components, multi-IC):** Expect 50-70% accuracy. Too many
  connections for reliable visual extraction without preprocessing.

The key insight from Auto-SPICE research: **LLM accuracy drops significantly for circuits
with more than ~10 components** unless the image is pre-annotated with net labels.

### 7.2 Fundamental Limitations

1. **No image preprocessing available.** Unlike SINA/Auto-SPICE, we cannot run YOLO or
   CCL on the PDF before sending to the LLM. We rely entirely on Gemini's native vision
   capabilities. This is the single biggest limitation.

2. **PDF rendering quality.** Gemini sees a rendered version of the PDF, not vector
   graphics. Small details (junction dots, pin numbers) may be lost at lower resolutions.

3. **Schematic drawing conventions vary.** Different manufacturers use different symbol
   styles, line weights, and annotation conventions. Chinese datasheets (TP4056) often
   have different conventions than TI datasheets (TPS5430).

4. **Multi-page circuits.** Some datasheets spread the application circuit across multiple
   pages or have critical annotations on a different page than the circuit. The PDF
   splitting logic in `extract.rs` may separate these.

5. **Equation-based component values.** Some components have calculated values
   (e.g., "R1 = VREF * R2 / (VOUT - VREF)"). The extraction should capture the
   recommended value from the specific example circuit, not the formula.

### 7.3 Failure Modes to Anticipate

| Failure Mode | Impact | Mitigation |
|---|---|---|
| Missing a component entirely | Incomplete circuit | Prompt asks for explicit count verification |
| Wrong net assignment (pin connected to wrong net) | Broken circuit | Self-verification checklist in prompt |
| Missing a connection at a junction | Open circuit at a critical node | Emphasize junction detection in prompt |
| Confusing two circuits on the same page | Mixed-up topology | Separate circuit extraction with page/description anchoring |
| Inventing connections not in the schematic | Short circuit in design | Anti-hallucination preamble |
| Wrong polarity on polarized component | Reversed diode/cap | Explicit polarity extraction step |

---

## 8. Alternative Approaches

### 8.1 Approach A: Full Netlist Extraction (Proposed Above)

Extract complete circuit topology as a graph of nets and component-pin connections.

**Pros:** Complete information for automated schematic generation; self-contained
**Cons:** Highest extraction complexity; most likely to have errors in connectivity

### 8.2 Approach B: Enhanced BOM + Critical Connections Only

Extend the existing reference-design format to capture only the "important" connections
that an LLM agent couldn't easily figure out from context.

```json
{
  "components": [ /* same as current format */ ],
  "critical_connections": [
    {
      "description": "Bootstrap capacitor between BOOT and PH pins",
      "net_name": "BOOT_CAP",
      "connections": [
        { "component": "C2", "pin": "1", "connects_to": "U1.BOOT" },
        { "component": "C2", "pin": "2", "connects_to": "U1.PH" }
      ]
    },
    {
      "description": "Feedback divider",
      "connections": [
        { "from": "VOUT", "through": "R1", "to": "U1.VSENSE" },
        { "from": "U1.VSENSE", "through": "R2", "to": "GND" }
      ]
    }
  ]
}
```

**Pros:** Lower extraction complexity; higher accuracy for what it does extract;
backward-compatible with existing format
**Cons:** Incomplete -- the LLM agent still needs to infer some connections; harder to
validate completeness

### 8.3 Approach C: Two-Phase Extraction

First extract the BOM (existing task), then make a second LLM call providing the BOM
as context along with the circuit image, asking only for connectivity.

**Pros:** Each LLM call has a simpler task; BOM provides grounding for connectivity
**Cons:** Two API calls (double the cost); requires orchestrating dependent extractions

### 8.4 Approach D: SPICE-Format Netlist

Instead of a custom JSON format, ask the LLM to produce a SPICE netlist directly:

```
* TPS5430 Typical Application - 12V to 5V
V_IN VIN 0 DC 12
C1 VIN 0 10u
C2 BOOT SW 0.01u
L1 SW VOUT 15u
D1 0 SW B340A
C3 VOUT 0 220u
R1 VOUT FB 10k
R2 FB 0 3.24k
X_U1 BOOT VIN PH GND VSENSE ENA TPS5430
```

**Pros:** Well-established format; LLMs have been trained on SPICE netlists; directly
verifiable with SPICE simulators
**Cons:** Less structured for our downstream use case (need to parse SPICE back to JSON);
SPICE pin ordering varies; doesn't capture metadata (voltage ratings, dielectric types)

### 8.5 Recommendation

**Start with Approach A (full netlist in JSON)** with a fallback mechanism: if the
extraction fails validation (e.g., pins not accounted for, obviously broken topology),
fall back to the existing reference-design format plus whatever connectivity was
successfully extracted.

Consider **Approach C (two-phase)** as a future enhancement if single-pass accuracy
is insufficient. The first pass (BOM) is already working; adding a connectivity-only
second pass is a natural extension.

---

## 9. Integration with Downstream Schematic Capture Workflow

### 9.1 From Extracted Netlist to .schdoc-spec

The extracted JSON netlist maps directly to `.schdoc-spec` constructs:

| Extracted Data | .schdoc-spec Construct |
|---|---|
| `components[*]` | `component "U1" { lib_reference: ... }` |
| `nets[*]` where type=ground | `power_object "GND" { orientation: 180 }` |
| `nets[*]` where type=power_input | `power_object "VIN" { style: bar }` |
| `nets[*]` where type=power_output | `power_object "VOUT" { style: bar }` |
| `nets[*]` where type=internal | `net_label "SW" { ... }` + wires |
| `nets[*].connections` | `wire { vertices: [...] }` or `net NAME { pins: [...] }` |
| `notes[*]` | `note { text: "..." }` |

### 9.2 Workflow Integration

The schematic capture agent (Phase 4) would use this data as follows:

1. Read the extracted circuit JSON
2. For each component, look up its schematic symbol in the SchLib
3. Place components on the schematic sheet (using symbol pin positions)
4. For each net, create the appropriate power objects or net labels
5. Wire components together using the net connection lists
6. Add notes from the extraction

The `net` block syntax in `.schdoc-spec` is particularly well-suited:

```
// Generated from extracted circuit data
net SW {
    pins: ["U1.PH", "C2.2", "L1.1", "D1.K"]
}

net VOUT {
    pins: ["L1.2", "C3.+", "R1.1"]
}
```

This is a direct 1:1 mapping from the extraction output.

### 9.3 Cross-Validation Opportunity

The extracted circuit netlist can be cross-validated against:
- **Pinout extraction:** Verify IC pin names match
- **Reference-design extraction:** Verify component list and values match
- **Characteristics extraction:** Verify voltage ratings are compatible

This cross-validation is a significant advantage of having structured extraction data.

---

## 10. Implementation Plan

### Phase 1: New Extraction Task

1. Add `ApplicationCircuit` variant to `ExtractTask` enum in `extract.rs`
2. Create `prompts/extract-application-circuit.md` with the prompt
3. Add JSON schema in `prompts.rs` (kept shallow per Gemini limits)
4. Wire up the new task in the CLI

### Phase 2: Prompt Development and Testing

1. Write the extraction prompt following patterns from existing tasks
2. Test against 3-4 representative datasheets:
   - AP2112 (simple LDO -- should be easy)
   - TPS5430 (buck converter -- medium complexity)
   - TP4056 (multiple circuits -- tests variant handling)
   - DRV8871 (external load -- tests abstract components)
3. Iterate on the prompt based on extraction accuracy
4. Add verification logic in the prompt

### Phase 3: Validation and Integration

1. Build a validation script that checks extracted netlists for:
   - All component pins assigned to nets
   - No duplicate pin assignments
   - All nets have >= 2 connections (except external I/O)
   - IC pin names match pinout extraction data
2. Test the full pipeline: datasheet -> extraction -> .schdoc-spec generation
3. Document the extraction task in the CLI reference

### Estimated Effort

- Prompt development and testing: 4-6 hours (most of the work)
- Code changes (new task, schema, wiring): 1-2 hours
- Documentation: 1 hour
- Total: 6-9 hours

---

## 11. Open Questions

1. **Should this replace or supplement the existing `reference-design` task?**
   The existing task captures component selection metadata (ESR requirements, voltage
   ratings, package sizes) that the netlist extraction might not focus on. Recommendation:
   keep both, with the circuit extraction focused on topology and the reference-design
   focused on component specifications.

2. **How to handle multiple application circuits per datasheet?**
   Extract all of them as separate entries? Or only the "typical" one? Recommendation:
   extract all, tag with circuit_type (typical/alternative), let the downstream agent
   choose.

3. **Should the IC itself be a component in the netlist?**
   Yes -- the IC is the central component and its pins define most net names. Including
   it makes the netlist self-contained.

4. **What about components not in the application circuit but mentioned in the text?**
   E.g., "An additional 0.1uF ceramic bypass capacitor can also be used." These should be
   captured as optional components with `is_required: false`.

5. **Should we attempt to extract spatial/layout information?**
   E.g., relative positions of components in the schematic. This could help with
   schematic placement, but adds significant extraction complexity. Recommendation:
   defer to a future enhancement.

---

## References

- [SINA: A Circuit Schematic Image-to-Netlist Generator Using Artificial Intelligence](https://arxiv.org/html/2601.22114v1) (2026)
- [Auto-SPICE / Masala-CHAI: Leveraging LLMs for Dataset Creation via Automated SPICE Netlist Extraction from Analog Circuit Diagrams](https://arxiv.org/html/2411.14299v1) (2024)
- [Netlistify: Transforming Circuit Schematics into Netlists with Deep Learning](https://research.nvidia.com/labs/electronic-design-automation/papers/netlistify_mlcad25.pdf) (NVIDIA, 2025)
- [Schemato: An LLM for Netlist-to-Schematic Conversion](https://arxiv.org/html/2411.13899v2) (2024)
- [EEschematic: Multimodal-LLM Based AI Agent for Schematic Generation of Analog Circuit](https://www.researchgate.net/publication/396715544) (2025)
- [CircuitLM: A Multi-Agent LLM-Aided Design Framework for Generating Circuit Schematics from Natural Language Prompts](https://arxiv.org/html/2601.04505v1) (2026)
- [Digitizing images of electrical-circuit schematics](https://pubs.aip.org/aip/aml/article/2/1/016109/3132693/) (AIP, 2024)
- [Testing Generative AI for Circuit Board Design](https://blog.jitx.com/jitx-corporate-blog/testing-generative-ai-for-circuit-board-design) (JITX, 2024)
- [Gemini Structured Output Documentation](https://ai.google.dev/gemini-api/docs/structured-output)
