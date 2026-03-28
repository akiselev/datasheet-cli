# Fab House Capabilities & Impedance Calculation — Research Note

**Date:** 2026-03-17
**Context:** datasheet-cli helps LLM agents design PCBs. After schematic capture, the
agent needs to set up PCB design rules (min trace width, min drill, min spacing, etc.)
based on the target fab house capabilities, and calculate trace widths for controlled
impedance (USB, Ethernet, DDR).

---

## 1. Fab House Capabilities Comparison

### 1.1 JLCPCB

**Source:** jlcpcb.com/capabilities, /impedance, /quote pages

| Parameter                | Value                                    |
| ------------------------ | ---------------------------------------- |
| Layers                   | 1--32                                    |
| Min trace width/space    | 3.5 mil (0.09 mm) advanced; 5 mil standard |
| Min drill (mechanical)   | 0.2 mm (additional charge < 0.3 mm)      |
| Min annular ring         | 0.15 mm (6 mil) typical                  |
| Board thickness          | 0.2--3.2 mm                              |
| Copper weight (outer)    | 1 oz, 2 oz (up to 3 oz advanced)         |
| Copper weight (inner)    | 0.5 oz, 1 oz, 2 oz                       |
| Surface finishes         | HASL, Lead-free HASL, ENIG               |
| Solder mask colors       | 7 options (green, black, white, red, yellow, blue, purple) |
| Impedance control        | Yes, +/-10% tolerance                    |
| Via types                | Through-hole, blind/buried (advanced)    |
| Max board size           | ~400 x 500 mm (varies by layer count)    |
| Min board size           | 5 x 5 mm                                |
| Castellated holes        | Min 0.5 mm diameter                      |
| Board outline tolerance  | +/-0.2 mm (CNC routing)                  |
| FR-4 Tg options          | 130--140 C (low), 150--160 C (std), >170 C (high) |
| FR-4 Er (core)           | 4.6 (at 1 MHz)                           |
| FR-4 loss tangent        | ~0.02 (at 1 MHz)                         |

### 1.2 PCBWay

**Source:** pcbway.com/capabilities.html

| Parameter                | Value                                    |
| ------------------------ | ---------------------------------------- |
| Layers                   | 1--14 standard; up to 24+ advanced       |
| Min trace width/space    | 4 mil (0.1 mm)                           |
| Min drill                | 0.15 mm                                  |
| Min annular ring         | 6 mil (0.15 mm)                          |
| Board thickness          | 0.2--3.2 mm (up to 4.5 mm available)     |
| Copper weight (outer)    | 1--8 oz (35--280 um)                     |
| Copper weight (inner)    | 1--4 oz (35--140 um)                     |
| Surface finishes         | HASL, ENIG, OSP, hard gold, immersion Ag, immersion Sn, ENEPIG |
| Solder mask colors       | Green, red, yellow, blue, white, black, matt green, matte black, purple |
| Impedance control        | Yes, +/-10% standard; +/-5 Ohm for <=50 Ohm |
| Via types                | Through-hole, blind/buried, via-in-pad   |
| Max board size (1-2L)    | 600 x 1200 mm                            |
| Max board size (multi)   | 560 x 1150 mm                            |
| Min board size           | 3 x 3 mm                                |
| Plated half-holes        | Min 0.4 mm diameter                      |
| Board outline tolerance  | +/-0.2 mm (CNC); +/-0.5 mm (V-score)    |
| Hole position tolerance  | +/-0.075 mm                              |
| PTH hole tolerance       | +/-0.08 mm                               |
| NPTH hole tolerance      | +/-0.05 mm                               |

### 1.3 OSH Park

**Source:** docs.oshpark.com/services/

| Parameter                | 2-Layer              | 4-Layer              | 6-Layer              |
| ------------------------ | -------------------- | -------------------- | -------------------- |
| Material                 | FR-4 (Tg 175)       | FR408-HR (Tg 190)   | FR408-HR (Tg 190)   |
| Dielectric constant      | 4.5 (10 MHz)        | 3.61 (1 GHz)        | 3.61 (1 GHz)        |
| Thickness                | 63 mil (1.6 mm)     | 63 mil (1.6 mm)     | 63 mil (1.6 mm)     |
| Min trace width/space    | 6 mil (0.15 mm)     | 5 mil (0.13 mm)     | 5 mil (0.13 mm)     |
| Min drill                | 10 mil (0.254 mm)   | 10 mil (0.254 mm)   | 8 mil (0.2 mm)      |
| Min annular ring         | 5 mil (0.127 mm)    | 4 mil (0.1 mm)      | 4 mil (0.1 mm)      |
| Copper (outer)           | 1 oz                | 1 oz                | 1 oz                |
| Copper (inner)           | N/A                 | 0.5 oz              | 0.5 oz              |
| Surface finish           | ENIG                | ENIG                | ENIG                |
| Solder mask              | Purple              | Purple              | Purple              |
| Blind/buried vias        | N/A                 | No                  | No                  |
| Max board size           | 16 x 22 in          | 16 x 22 in          | 16 x 22 in          |
| Board edge keepout       | 15 mil (0.38 mm)    | 15 mil (0.38 mm)    | 15 mil (0.38 mm)    |
| Pricing model            | $5/sq.in/set of 3   | $10/sq.in/set of 3  | Per-quote            |

### 1.4 Comparison Summary

For prototype PCB design, these are the **safe, broadly-compatible design rules** an
agent should default to when no specific fab is chosen:

| Rule                     | Conservative Default | Aggressive (JLCPCB) |
| ------------------------ | -------------------- | -------------------- |
| Min trace width          | 6 mil (0.15 mm)     | 3.5 mil (0.09 mm)   |
| Min trace spacing        | 6 mil (0.15 mm)     | 3.5 mil (0.09 mm)   |
| Min drill                | 0.3 mm (12 mil)     | 0.2 mm (8 mil)      |
| Min annular ring         | 6 mil (0.15 mm)     | 5 mil (0.13 mm)     |
| Min via (drill + ring)   | 0.6 mm / 0.3 mm     | 0.45 mm / 0.2 mm    |
| Board outline tolerance  | +/-0.2 mm           | +/-0.2 mm           |

---

## 2. Available Data Sources

### 2.1 Web Pages (Unstructured)

All three fab houses publish capabilities as human-readable web pages. None publish
a machine-readable (JSON/XML) capabilities document on their public websites.

| Fab House | Capabilities URL                                  | Format     |
| --------- | ------------------------------------------------- | ---------- |
| JLCPCB    | jlcpcb.com/capabilities/pcb-capabilities          | HTML (JS-rendered, hard to scrape) |
| JLCPCB    | jlcpcb.com/impedance                              | HTML (has stackup data) |
| PCBWay    | pcbway.com/capabilities.html                      | HTML (well-structured, scrapeable) |
| OSH Park  | docs.oshpark.com/services/                        | HTML (clean markdown-like) |

JLCPCB's pages are heavily JavaScript-rendered (Nuxt.js SPA). The actual capability
numbers are often not in the initial HTML — they're loaded dynamically. PCBWay's
capabilities page is more traditional HTML with data present in the initial response.

### 2.2 APIs

**JLCPCB API** (api.jlcpcb.com):
- Offers PCB API, Stencil API, 3D Printing API, Components API
- PCB API focuses on ordering/procurement workflow, not capability queries
- Requires application for API access (enterprise-focused)
- No documented endpoint for "give me your design rules" or "check my DFM"
- No stackup query endpoint identified

**JLCDFM** (jlcdfm.com):
- Free web-based DFM analysis tool
- Uploads Gerber files for 30+ point DFM checklist
- GUI only — no API for programmatic DFM checking
- Useful for validation after design, not for setting up rules before design

**PCBWay:**
- No public API for capabilities or DFM
- Offers a GitHub repo with design rule files for KiCad, Altium, Eagle, Allegro:
  https://github.com/pcbway/PCBWay-Design-Rules (16 stars)
- These are DRC template files, not structured capability data

**OSH Park:**
- No API
- Clean documentation pages that are easy to scrape

**Eurocircuits:**
- No public API identified
- Publishes design guidelines as web articles (metric units)

### 2.3 Structured Files from Fab Houses

**PCBWay-Design-Rules GitHub repo** contains DRC files for:
- KiCad (.kicad_dru)
- Altium (.RUL or similar)
- Eagle (.dru)
- Allegro

These are the closest thing to machine-readable fab specs but are templates that
"can only be performed by the user" per the README disclaimer.

### 2.4 Summary: No Good API Exists

The PCB fabrication industry does not have a standard for publishing capabilities
in a machine-readable format. The practical options are:

1. **Hardcode known fab specs** — Maintain a curated database of fab house
   capabilities in the CLI itself. This is the most reliable approach.
2. **Extract from web pages** — Fragile, breaks when pages change, and JLCPCB
   pages are SPA-rendered.
3. **Parse EDA design rule files** — PCBWay publishes these; could import them.
4. **LLM extraction from web pages** — Use datasheet-cli's extraction approach
   on fab capability pages. More resilient to format changes but expensive.

---

## 3. JLCPCB Stackup Data

JLCPCB publishes detailed stackup configurations on their impedance page. This
data is critical for impedance calculations.

### 3.1 Dielectric Constants (JLCPCB Materials)

| Material       | Er    | Notes                        |
| -------------- | ----- | ---------------------------- |
| Prepreg 7628   | 4.40  | Most common, cheapest        |
| Prepreg 3313   | 4.10  | Thinner prepreg option       |
| Prepreg 1080   | 3.91  | Thinnest prepreg option      |
| Prepreg 2116   | 4.16  | Mid-range prepreg            |
| Core           | 4.60  | Standard FR-4 core           |
| Solder mask    | 3.80  | For solder mask coating      |

### 3.2 Solder Mask Geometry

| Parameter                        | Value    |
| -------------------------------- | -------- |
| Coating above substrate (C1)     | 1.2 mil  |
| Coating above trace (C2)         | 0.6 mil  |
| Coating between traces (C3)      | 1.2 mil  |

### 3.3 Four-Layer Stackup Options (17 configurations)

JLCPCB offers 17 four-layer configurations. Key examples:

**JLC04161H-7628** (Standard, lowest cost):
```
Layer         Material      Thickness (mm)   Cu weight
─────────────────────────────────────────────────────
Top copper    Cu            0.035            1 oz
Prepreg       7628          0.2104           Er=4.4
Inner L2 Cu   Cu            0.0152           0.5 oz
Core          FR-4          ~1.065           Er=4.6
Inner L3 Cu   Cu            0.0152           0.5 oz
Prepreg       7628          0.2104           Er=4.4
Bottom copper Cu            0.035            1 oz
─────────────────────────────────────────────────────
Total: ~1.6 mm
```

**JLC04161H-1080** (Tighter coupling, thinner prepreg):
```
Layer         Material      Thickness (mm)   Cu weight
─────────────────────────────────────────────────────
Top copper    Cu            0.035            1 oz
Prepreg       1080          0.0764           Er=3.91
Inner L2 Cu   Cu            0.0152           0.5 oz
Core          FR-4          ~1.265           Er=4.6
Inner L3 Cu   Cu            0.0152           0.5 oz
Prepreg       1080          0.0764           Er=3.91
Bottom copper Cu            0.035            1 oz
─────────────────────────────────────────────────────
Total: ~1.6 mm
```

Available total thicknesses for 4L: 0.8, 1.0, 1.2, 1.6, 2.0 mm.
Outer copper options: 1 oz (0.035 mm), 2 oz (0.070 mm).
Inner copper options: 0.5 oz (0.0152 mm), 1 oz (0.035 mm), 2 oz (0.070 mm).

### 3.4 Six-Layer Stackup Options (12 configurations)

Example: **JLC06161H-3313** (1.6 mm):
```
Layer         Material      Thickness (mm)   Cu weight
─────────────────────────────────────────────────────
Top copper    Cu            0.035            1 oz
Prepreg       3313          0.0994           Er=4.1
Inner L2 Cu   Cu            0.0152           0.5 oz
Core          FR-4          0.55             Er=4.6
Inner L3 Cu   Cu            0.0152           0.5 oz
Prepreg       2116          0.1088           Er=4.16
Inner L4 Cu   Cu            0.0152           0.5 oz
Core          FR-4          0.55             Er=4.6
Inner L5 Cu   Cu            0.0152           0.5 oz
Prepreg       3313          0.0994           Er=4.1
Bottom copper Cu            0.035            1 oz
─────────────────────────────────────────────────────
Total: ~1.6 mm
```

Available total thicknesses for 6L: 1.2, 1.6, 2.0 mm.

### 3.5 OSH Park Stackup Data

**2-Layer:**
- 60 mil (1.524 mm) core, FR-4 (Kingboard KB6167F), Er=4.5 at 10 MHz
- 1 oz copper each side (1.4 mil = 0.035 mm)
- 0.6 mil solder resist + 0.6 mil silkscreen per side
- Total ~63 mil (1.6 mm)

**4-Layer (FR408-HR, Er=3.61 at 1 GHz):**
```
Layer         Material      Thickness (mil)  Cu weight
─────────────────────────────────────────────────────
Silkscreen    -             0.6              -
Solder resist -             0.6              -
Top copper    Cu            clad+plated      1 oz
Prepreg       FR408-HR      7.87             Er=3.61
Inner L2      Cu            -                0.5 oz
Core          FR408-HR      39               Er=3.61
Inner L3      Cu            -                0.5 oz
Prepreg       FR408-HR      7.87             Er=3.61
Bottom copper Cu            clad+plated      1 oz
Solder resist -             0.6              -
Silkscreen    -             0.6              -
─────────────────────────────────────────────────────
Total: ~63 mil (1.6 mm)
```

---

## 4. Impedance Calculation Formulas

### 4.1 Overview

The standard impedance formulas for PCB traces are well-established in IPC-2141
("Design Guide for High-Speed Controlled Impedance Circuit Boards"). The key
topologies and their formulas are implemented in our existing `pcb-toolkit` crate.

### 4.2 Microstrip (Outer Layer, Single-Ended)

**Reference:** Hammerstad & Jensen, IEEE MTT-S 1980.

The trace is on the outer layer with dielectric below and air above.

**Effective dielectric constant (Hammerstad-Jensen):**
```
For u = W_eff / H:

If u <= 1:
    f(u) = (1 + 12/u)^(-0.5) + 0.04 * (1 - u)^2

If u > 1:
    f(u) = (1 + 12/u)^(-0.5)

Er_eff = (Er + 1)/2 + (Er - 1)/2 * f(u)
```

**Characteristic impedance:**
```
If u <= 1 (narrow trace):
    Z0 = (60 / sqrt(Er_eff)) * ln(8H/W_eff + W_eff/(4H))

If u > 1 (wide trace):
    Z0 = (120*pi / sqrt(Er_eff)) / (u + 1.393 + 0.667*ln(u + 1.444))
```

**Thickness correction (effective width):**
```
If W/H >= pi/2:
    dW = (T/pi) * (1 + ln(2H/T))
Else:
    dW = (T/pi) * (1 + ln(4*pi*W/T))

W_eff = W + dW
```

Where:
- W = trace width (mils)
- H = dielectric height to ground plane (mils)
- T = copper thickness (mils)
- Er = relative permittivity of substrate

### 4.3 Centered Stripline (Inner Layer, Single-Ended)

**Reference:** Cohn/Wadell.

The trace is centered between two ground planes, fully embedded in dielectric.

```
Z0 = (60 / sqrt(Er)) * ln(1.9 * (2H + T) / (0.8W + T))

Er_eff = Er  (no air interface)
```

Where:
- H = distance from trace to each ground plane (half the total dielectric)
- W = trace width
- T = copper thickness
- Er = substrate permittivity

### 4.4 Edge-Coupled Differential Pair (External/Microstrip)

**Reference:** IPC-2141 approximation.

```
Z0_single = (87 / sqrt(Er + 1.41)) * ln(5.98*H / (0.8*W + T))

Z_odd  = Z0_single * (1 - 0.48 * exp(-0.96 * S / H))
Z_even = Z0_single^2 / Z_odd
Z_diff = 2 * Z_odd

Coupling coefficient:
    Kb = (Z_even - Z_odd) / (Z_even + Z_odd)
```

Where S = gap between the two traces.

### 4.5 Edge-Coupled Differential Pair (Internal/Stripline)

Same coupling correction applied to the Cohn stripline Z0:

```
Z0_single = (60 / sqrt(Er)) * ln(1.9 * (2H + T) / (0.8W + T))

Z_odd  = Z0_single * (1 - 0.48 * exp(-0.96 * S / H))
Z_even = Z0_single^2 / Z_odd
Z_diff = 2 * Z_odd
```

### 4.6 Coplanar Waveguide (CPW over Ground)

**Reference:** Wadell, "Transmission Line Design Handbook", 1991.

Uses elliptic integral ratios (Hilberg approximation):

```
k  = W / (W + 2*G)
k3 = tanh(pi*W/(4H)) / tanh(pi*(W+2G)/(4H))

Er_eff = 1 + (Er-1)/2 * (1/elliptic_ratio(k)) * elliptic_ratio(k3)
Z0     = 30*pi / (sqrt(Er_eff) * elliptic_ratio(k))

Where elliptic_ratio(k) = K(k)/K(k'):
    If k <= 1/sqrt(2):
        K(k)/K(k') = pi / ln(2*(1+sqrt(k'))/(1-sqrt(k')))
    If k > 1/sqrt(2):
        K(k)/K(k') = (1/pi) * ln(2*(1+sqrt(k))/(1-sqrt(k)))
```

Where G = gap between center conductor and coplanar ground.

### 4.7 Derived Quantities

From Z0 and Er_eff, we derive:
```
Propagation delay: Tpd = sqrt(Er_eff) / c   [ps/in, c = 11.803 in/ns]
Inductance:        Lo = Z0 * Tpd            [nH/in]
Capacitance:       Co = Tpd / Z0            [pF/in]
```

---

## 5. Common Impedance Targets

These are the standard impedance requirements for common digital interfaces:

| Interface        | Type                 | Target Impedance | Tolerance |
| ---------------- | -------------------- | ---------------- | --------- |
| USB 2.0          | Differential pair    | 90 Ohm           | +/-10%    |
| USB 3.x          | Differential pair    | 85 Ohm           | +/-10%    |
| Ethernet 10/100  | Differential pair    | 100 Ohm          | +/-10%    |
| Gigabit Ethernet | Differential pair    | 100 Ohm          | +/-10%    |
| DDR3 / DDR4      | Single-ended         | 40--60 Ohm       | +/-10%    |
| DDR3 / DDR4      | Differential (CLK)   | 80--120 Ohm      | +/-10%    |
| HDMI             | Differential pair    | 100 Ohm          | +/-10%    |
| PCIe             | Differential pair    | 85 Ohm           | +/-10%    |
| SATA             | Differential pair    | 85 Ohm           | +/-15%    |
| LVDS             | Differential pair    | 100 Ohm          | +/-10%    |
| SPI / I2C / UART | Single-ended         | Not controlled   | N/A       |
| General GPIO     | Single-ended         | 50 Ohm           | +/-10%    |

---

## 6. Existing pcb-toolkit Crate

We already have a Rust crate `pcb-toolkit` (github.com/akiselev/pcb-toolkit) that
implements all the impedance formulas above. This is our own project.

### 6.1 What It Already Does

- **Microstrip** impedance (Hammerstad-Jensen) with thickness correction
- **Stripline** impedance (Cohn/Wadell centered)
- **Embedded microstrip** (buried trace below solder mask)
- **Coplanar waveguide** (Wadell with elliptic integrals)
- **Differential pair** impedance: 5 topologies
  - Edge-coupled external (surface microstrip diff pair)
  - Edge-coupled internal symmetric (centered stripline diff pair)
  - Edge-coupled internal asymmetric (offset stripline diff pair)
  - Edge-coupled embedded (buried microstrip diff pair)
  - Broadside-coupled (traces on different layers, overlapping)
- **45 built-in substrate materials** with Er, Tg, roughness factor
- **JSON output** mode for machine-readable results
- **Unit parsing** (10mil, 0.254mm, 1GHz, 10nF)
- Validated against Saturn PCB Toolkit v8.44

### 6.2 What It Does NOT Do (Yet)

- No "inverse" calculation: given target impedance + stackup, solve for trace width
- No fab-house-specific stackup database (JLCPCB stackups, OSH Park stackups)
- No design rule / DFM capability database
- No integration with datasheet-cli

### 6.3 Architecture

```
pcb-toolkit/
  crates/
    pcb-toolkit/        # Core library
      src/
        impedance/      # microstrip, stripline, embedded, coplanar
        differential/   # 5 differential pair topologies
        materials.rs    # 45 substrate materials database
        units.rs        # Unit parsing and conversion
        ...
    pcb-toolkit-cli/    # CLI wrapper
```

Dual-licensed Apache-2.0 / MIT. Rust edition 2024 (requires 1.85+).

---

## 7. Existing DRC Extraction in datasheet-cli

The `extract drc-rules` command in datasheet-cli extracts PCB design constraints
from **component datasheets** — not from fab house specs. The extraction prompt
(`prompts/extract-drc-rules.md`) maps datasheet layout guidelines to these rule types:

- Clearance, Width, RoutingVias, PlaneConnect, PolygonConnect
- SolderMaskExpansion, PasteMaskExpansion, Height
- DiffPairsRouting, MaxMinImpedance, Length
- NetAntennae, FanoutControl, ComponentClearance

This is **complementary** to fab capabilities, not a replacement:

| Source               | Provides                                       |
| -------------------- | ---------------------------------------------- |
| Datasheet DRC rules  | Component-specific: thermal vias, diff pairs,  |
|                      | creepage, paste aperture, specific net rules    |
| Fab house specs      | Global manufacturing limits: min trace, min     |
|                      | drill, min spacing, available layer counts,     |
|                      | stackup options, impedance tolerance            |

Both are needed: fab specs define the floor (what's manufacturable), and component
DRC rules define additional constraints on top of that.

---

## 8. Proposed CLI Commands

### 8.1 Fab Capability Database

```bash
# List known fab houses
datasheet fab list

# Show capabilities for a specific fab
datasheet fab show jlcpcb [--json]

# Show capabilities with specific options
datasheet fab show jlcpcb --layers 4 --thickness 1.6mm [--json]

# Show stackup options for a fab
datasheet fab stackups jlcpcb --layers 4 [--json]

# Show a specific stackup
datasheet fab stackup jlcpcb JLC04161H-7628 [--json]

# Generate design rules for a fab (outputs to a format usable by altium-cli)
datasheet fab rules jlcpcb [--margin conservative|standard|aggressive] [--json]
```

### 8.2 Impedance Calculation (Delegate to pcb-toolkit)

Rather than reimplementing impedance in datasheet-cli, delegate to pcb-toolkit:

```bash
# Calculate trace width for target impedance on a specific stackup
datasheet impedance solve \
    --target 90ohm \
    --type differential \
    --stackup jlcpcb:JLC04161H-7628 \
    --layer top \
    [--json]

# Output:
{
    "target_impedance": 90.0,
    "type": "differential",
    "stackup": "JLC04161H-7628",
    "signal_layer": "Top",
    "reference_layer": "L2",
    "dielectric_height_mm": 0.2104,
    "dielectric_er": 4.4,
    "copper_thickness_mm": 0.035,
    "solution": {
        "trace_width_mm": 0.127,
        "trace_width_mil": 5.0,
        "trace_spacing_mm": 0.127,
        "trace_spacing_mil": 5.0,
        "actual_impedance": 89.7,
        "tolerance_band": [80.7, 98.7]
    }
}
```

Alternatively, since pcb-toolkit already has a CLI with `--json`, the agent could
call pcb-toolkit directly. The question is whether to:
1. Add pcb-toolkit as a Rust dependency to datasheet-cli
2. Shell out to pcb-toolkit CLI
3. Keep them separate and let the agent call both

### 8.3 Proposed Output Format for Fab Rules

```json
{
    "fab_house": "jlcpcb",
    "process": "standard",
    "capabilities": {
        "layers": {
            "options": [1, 2, 4, 6, 8, 10, 12, 14, 16, 20, 32],
            "default": 2
        },
        "trace": {
            "min_width_mm": 0.09,
            "min_width_mil": 3.5,
            "recommended_min_width_mm": 0.127,
            "recommended_min_width_mil": 5.0
        },
        "spacing": {
            "min_mm": 0.09,
            "min_mil": 3.5,
            "recommended_min_mm": 0.127,
            "recommended_min_mil": 5.0
        },
        "drill": {
            "min_mechanical_mm": 0.2,
            "min_mechanical_mil": 7.87,
            "recommended_min_mm": 0.3,
            "recommended_min_mil": 11.81,
            "surcharge_below_mm": 0.3
        },
        "annular_ring": {
            "min_mm": 0.13,
            "min_mil": 5.0
        },
        "board": {
            "thickness_options_mm": [0.4, 0.6, 0.8, 1.0, 1.2, 1.6, 2.0, 2.4, 3.2],
            "max_size_mm": [400, 500],
            "min_size_mm": [5, 5]
        },
        "copper_weight": {
            "outer_oz": [1, 2],
            "inner_oz": [0.5, 1, 2]
        },
        "impedance_control": {
            "available": true,
            "tolerance_percent": 10
        },
        "surface_finish": ["HASL", "LeadFree-HASL", "ENIG"]
    },
    "stackups": {
        "4-layer": [
            {
                "id": "JLC04161H-7628",
                "total_thickness_mm": 1.6,
                "layers": [
                    {"name": "Top", "type": "copper", "thickness_mm": 0.035, "weight_oz": 1.0},
                    {"name": "Prepreg", "type": "dielectric", "thickness_mm": 0.2104, "er": 4.4, "material": "7628"},
                    {"name": "L2", "type": "copper", "thickness_mm": 0.0152, "weight_oz": 0.5},
                    {"name": "Core", "type": "dielectric", "thickness_mm": 1.065, "er": 4.6, "material": "FR-4"},
                    {"name": "L3", "type": "copper", "thickness_mm": 0.0152, "weight_oz": 0.5},
                    {"name": "Prepreg", "type": "dielectric", "thickness_mm": 0.2104, "er": 4.4, "material": "7628"},
                    {"name": "Bottom", "type": "copper", "thickness_mm": 0.035, "weight_oz": 1.0}
                ],
                "cost_tier": "lowest",
                "notes": "Standard 7628 prepreg, recommended for most designs"
            }
        ]
    }
}
```

---

## 9. Recommended Implementation Approach

### 9.1 Hardcode Fab Specs (Phase 1)

**Recommendation: Hardcode a curated database of fab house capabilities.**

Rationale:
- Fab capabilities change rarely (maybe once a year at most)
- Web scraping is fragile, especially for JLCPCB (Nuxt.js SPA)
- No useful APIs exist for querying capabilities
- The data set is small (3-5 fab houses, ~20 parameters each)
- Manual verification is critical — wrong DFM rules waste money

Implementation:
1. Define a `FabHouse` struct with all capability fields
2. Define a `Stackup` struct with per-layer material data
3. Hardcode JLCPCB, PCBWay, OSH Park as built-in data
4. Allow user-defined fab specs via TOML/JSON files
5. Provide a `datasheet fab show` command to inspect specs

### 9.2 Impedance Integration (Phase 2)

**Recommendation: Add pcb-toolkit as a Rust library dependency.**

Rationale:
- pcb-toolkit is our own crate, dual-licensed Apache-2.0/MIT
- Adding it as a dependency avoids shelling out to another CLI
- We get all the validated impedance formulas without reimplementing
- The crate has minimal dependencies (thiserror + serde)

Implementation:
1. Add `pcb-toolkit = "0.1"` to datasheet-cli's Cargo.toml
2. Implement a `datasheet impedance solve` command
3. The solve command does inverse calculation: binary search on trace width
   to achieve target impedance given a stackup
4. Use the stackup database from Phase 1 to look up dielectric heights and Er values

### 9.3 Inverse Impedance Solver

pcb-toolkit calculates impedance from geometry. We need the inverse: geometry
from impedance. The approach:

1. Binary search on trace width (W) for single-ended impedance
2. Binary search on trace width + spacing for differential pairs
3. Convergence criterion: |Z_calculated - Z_target| < 0.1 Ohm
4. Search range: 1 mil to 100 mil trace width
5. For differential pairs: additional search on spacing

This is straightforward to implement since all the impedance functions are
monotonically decreasing with trace width (wider trace = lower impedance).

### 9.4 Agent Workflow Integration

The intended workflow for the LLM agent (in Phase 5 of ee-template):

```
1. Agent selects fab house (default: JLCPCB)
2. `datasheet fab rules jlcpcb --json` -> global DFM constraints
3. For each controlled-impedance interface (USB, Ethernet, etc.):
   a. Agent knows target impedance from interface spec
   b. `datasheet impedance solve --target 90ohm --type differential
       --stackup jlcpcb:JLC04161H-7628 --layer top --json`
   c. Agent gets trace width and spacing
4. Agent combines global DFM rules + per-net impedance rules
5. Agent writes these into the .pcbdoc-spec design rules section
```

### 9.5 Future: LLM Extraction from Fab Pages (Phase 3)

If we want to support arbitrary fab houses without hardcoding, a future enhancement
could use Gemini-based extraction (like the existing datasheet extraction) on fab
capability web pages. This would:

1. Fetch the fab's capabilities page HTML
2. Send to Gemini with a structured extraction prompt
3. Output the same JSON format as the hardcoded database
4. Allow caching the result for future runs

This is lower priority since the hardcoded approach covers the 90% case.

### 9.6 File Structure

```
datasheet-cli/
  src/
    fab/
      mod.rs              # FabHouse, Stackup structs
      database.rs         # Hardcoded fab specs
      jlcpcb.rs           # JLCPCB-specific data and stackups
      pcbway.rs           # PCBWay-specific data
      oshpark.rs          # OSH Park-specific data
    impedance/
      mod.rs              # Impedance solve command
      solver.rs           # Inverse impedance solver (binary search)
    commands/
      fab.rs              # `datasheet fab` subcommands
      impedance.rs        # `datasheet impedance` subcommands
```

---

## 10. Open Questions

1. **Should pcb-toolkit be a workspace member of datasheet-cli, or a published
   crate dependency?** Currently pcb-toolkit is a separate repo. If we add it as a
   dependency, we need to publish it to crates.io or use a path/git dependency.

2. **Should we also store standard interface impedance targets?** The agent needs
   to know "USB 2.0 = 90 Ohm differential" — should this live in datasheet-cli
   or in ee-template's documentation?

3. **How do we handle stackup selection?** The agent needs to choose between
   17 four-layer JLCPCB stackups. Should the CLI recommend a stackup given
   impedance targets, or should the agent choose?

4. **Should the `fab` commands live in datasheet-cli or in a new crate?** The fab
   capability database is not really about "datasheets" — it might belong in
   pcb-toolkit or in a new `pcb-fab` crate.

5. **How to keep the hardcoded data up to date?** One option: a CI job that fetches
   fab pages periodically and diffs against the hardcoded values, alerting when
   they change.
