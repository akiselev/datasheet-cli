**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Signal Integrity Engineer and High-Speed Board Designer.

**Objective:** Identify high-speed communication interfaces (USB, Ethernet, PCIe, MIPI, DDR) and extract their specific routing constraints from the datasheet.

**Context:** The output will be used to configure PCB design rules and routing constraints for high-speed signals.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO high-speed interfaces exist in the device, respond with: `{"error": "No high-speed interfaces found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Identify ALL High-Speed Interfaces
Search for these interface types in pin descriptions, feature lists, and dedicated sections:

| Protocol Family | Keywords to Search |
|----------------|-------------------|
| USB | USB 2.0, USB 3.x, DP, DM, USBP, USBN, OTG |
| Ethernet | RGMII, RMII, MII, MDIO, MDC, TX+, TX-, RX+, RX- |
| PCIe | PCIe, PCI Express, Lane, PERST, REFCLK |
| MIPI | CSI, DSI, D-PHY, C-PHY, MIPI |
| DDR/Memory | DDR3, DDR4, LPDDR, DQ, DQS, DM, CK |
| LVDS | LVDS, differential, TMDS |
| CAN | CAN-FD, CANH, CANL |
| High-Speed Serial | SATA, DisplayPort, HDMI |

For EACH interface found, extract:

| Field | Requirement |
|-------|-------------|
| `protocol_name` | EXACT name and version (e.g., "USB 2.0 High Speed") |
| `associated_pins` | Array of ALL pin names for this interface |
| `net_class_type` | "Differential Pair", "Single-Ended", or "Bus" |
| `source_page` | 0-indexed page number |

### Step 2: Extract Impedance Requirements
For EACH interface, look for impedance specifications:

| Field | Requirement |
|-------|-------------|
| `differential_impedance_ohms` | Target differential impedance (e.g., 90) |
| `impedance_tolerance` | Tolerance (e.g., "+/-10%") |
| `single_ended_impedance_ohms` | Single-ended impedance if specified |

### Step 3: Extract Timing/Skew Requirements
Look for length matching and timing constraints:

| Field | Requirement |
|-------|-------------|
| `intra_pair_skew_tolerance` | P/N matching within pair (e.g., "Max 2.5mm") |
| `inter_pair_skew_tolerance` | Matching between pairs in a group |
| `max_trace_length_mm` | Maximum allowed trace length |
| `length_matching_group` | Which signals must be matched together |

### Step 4: Extract Termination Requirements
Document termination resistor requirements:

| Field | Requirement |
|-------|-------------|
| `termination_type` | "Internal", "External Series", "External Parallel" |
| `termination_value` | Resistor value if external |
| `termination_location` | "Source", "Load", "Both Ends" |
| `termination_notes` | Any special instructions |

### Step 5: Extract Via and Layer Constraints
Look for via restrictions and layer requirements:

| Field | Requirement |
|-------|-------------|
| `via_count_limit` | Maximum vias allowed |
| `via_type` | "Through-hole", "Blind", "Microvia" allowed |
| `reference_plane` | Required reference plane (GND, PWR) |
| `layer_restrictions` | Which layers can be used |

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List interfaces alphabetically by protocol_name
2. **Completeness:** Include ALL high-speed interfaces, even those without explicit routing rules
3. **Exactness:** Use EXACT impedance values from datasheet (don't assume standards)
4. **Units:** Always include units (ohms, mm, ps, etc.)

---

## IF DATA NOT FOUND

- If no impedance is specified: Set `"impedance_specified": false` and note standard values
- If no skew requirements exist: Set `"skew_requirements": null`
- If termination is not mentioned: Set `"termination_notes": "Not specified in datasheet"`
- If an interface exists but has no routing constraints: Include interface with `"routing_constraints_specified": false`

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [12, 45, 67],
  "interfaces": [
    {
      "protocol_name": "USB 2.0 High Speed",
      "associated_pins": ["USB_DP", "USB_DM"],
      "net_class_type": "Differential Pair",
      "source_page": 45,
      "constraints": {
        "differential_impedance_ohms": 90,
        "impedance_tolerance": "+/-10%",
        "single_ended_impedance_ohms": 45,
        "max_trace_length_mm": 150,
        "intra_pair_skew_tolerance": "Max 2.5mm (150ps)",
        "inter_pair_skew_tolerance": null,
        "via_count_limit": 2,
        "reference_plane": "GND"
      },
      "termination": {
        "termination_type": "Internal",
        "termination_value": null,
        "termination_notes": "Internal 45-ohm termination. Do not add external resistors."
      },
      "routing_notes": [
        "Keep USB traces away from switching power supply signals",
        "Route on outer layers with solid ground reference"
      ]
    },
    {
      "protocol_name": "RGMII Ethernet",
      "associated_pins": ["TXD[3:0]", "TX_CLK", "TX_EN", "RXD[3:0]", "RX_CLK", "RX_DV", "MDC", "MDIO"],
      "net_class_type": "Bus",
      "source_page": 67,
      "constraints": {
        "differential_impedance_ohms": null,
        "single_ended_impedance_ohms": 50,
        "impedance_tolerance": "+/-10%",
        "max_trace_length_mm": 100,
        "intra_pair_skew_tolerance": null,
        "inter_pair_skew_tolerance": "Match all data to clock within 50ps",
        "via_count_limit": null,
        "reference_plane": "GND"
      },
      "termination": {
        "termination_type": "External Series",
        "termination_value": "22-33 ohms",
        "termination_notes": "Place series resistors at source (IC side)"
      },
      "routing_notes": [
        "Length match all signals within 10mm",
        "MDIO and MDC do not require length matching"
      ]
    }
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL high-speed interfaces in the document are included
- [ ] Pin names match datasheet exactly
- [ ] Impedance values have units and tolerances
- [ ] Length/skew constraints include units (mm, ps, etc.)
- [ ] Source page numbers are 0-indexed and accurate
