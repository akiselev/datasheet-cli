**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior PCB Layout Designer and DFM (Design for Manufacturing) Specialist.

**Objective:** Extract physical layout rules, routing constraints, and placement strategies to configure PCB Design Rules (DRC) and guide component placement.

**Context:** The output will be used to create design rules and placement guidelines for PCB layout tools.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO layout guidelines exist, respond with: `{"error": "No layout guidelines found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Locate Layout Guidelines
Search for these sections:
- "Layout Guidelines"
- "PCB Layout Recommendations"
- "PCB Layout Examples"
- "Thermal Considerations"
- "Application Information"
- "Layout Tips"

Record the page number for each section found.

### Step 2: Extract Placement Rules
For EACH component placement constraint, extract:

| Field | Requirement |
|-------|-------------|
| `component` | Component name or type (e.g., "Input Capacitor", "Feedback Resistor") |
| `constraint_type` | "Distance", "Orientation", "Zone", "Grouping", "Connection" |
| `value_description` | EXACT constraint description |
| `priority` | "Critical", "High", "Medium", "Low" |
| `source_text` | EXACT quote from datasheet |
| `source_page` | 0-indexed page number |

### Step 3: Extract Routing Constraints
For EACH routing constraint, extract:

| Field | Requirement |
|-------|-------------|
| `net_type` | Net category (e.g., "Power", "Signal", "High-Speed", "Analog") |
| `net_names` | Specific net names if mentioned |
| `recommendation` | EXACT routing guidance |
| `trace_width` | Recommended trace width if specified |
| `via_requirements` | Via specifications if mentioned |
| `source_text` | EXACT quote from datasheet |
| `source_page` | 0-indexed page number |

### Step 4: Extract Grounding Strategy
Look for grounding recommendations:
- Ground plane requirements
- Star ground vs single-point ground
- Analog/digital ground separation
- Ground via placement

### Step 5: Extract Thermal Management Rules
Look for thermal constraints:
- Copper pour requirements
- Via requirements under thermal pads
- Heat sink recommendations
- Maximum operating temperature guidelines

### Step 6: Extract Layer Stack Requirements
Look for layer stack recommendations:
- Minimum layer count
- Reference plane requirements
- Signal layer assignments

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List placement rules by priority (Critical first, then High, Medium, Low)
2. **Ordering:** List routing constraints alphabetically by net_type
3. **Completeness:** Extract ALL constraints mentioned, not just typical ones
4. **Exactness:** Quote source text exactly (preserve wording)
5. **Units:** Always include units for dimensions (mm, mil, etc.)

---

## IF DATA NOT FOUND

- If no placement rules exist: Set `"placement_rules": []`
- If no routing constraints exist: Set `"routing_constraints": []`
- If no layer stackup guidance: Set `"layer_stackup_notes": null`
- If priority is not clear: Default to "Medium"
- If specific values are not given: Use the qualitative description (e.g., "minimize", "as close as possible")

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [12, 15, 18],
  "placement_rules": [
    {
      "component": "Thermal Pad",
      "constraint_type": "Connection",
      "value_description": "Must be soldered to PCB ground plane for thermal dissipation",
      "priority": "Critical",
      "source_text": "The exposed thermal pad must be soldered to a ground plane for proper thermal performance.",
      "source_page": 15
    },
    {
      "component": "Input Capacitor (Cin)",
      "constraint_type": "Distance",
      "value_description": "Place within 2mm of VIN and GND pins to minimize loop area",
      "priority": "High",
      "source_text": "Place input capacitor as close as possible to the VIN pin. Maximum distance: 2mm.",
      "source_page": 15
    },
    {
      "component": "Feedback Network",
      "constraint_type": "Zone",
      "value_description": "Keep feedback components away from switching node; minimize trace length",
      "priority": "High",
      "source_text": "Route feedback traces away from noisy switching node. Keep feedback resistor divider close to FB pin.",
      "source_page": 16
    },
    {
      "component": "Output Capacitor",
      "constraint_type": "Distance",
      "value_description": "Place close to VOUT pin to reduce output impedance",
      "priority": "Medium",
      "source_text": "Output capacitors should be placed close to the output pin.",
      "source_page": 15
    }
  ],
  "routing_constraints": [
    {
      "net_type": "Analog Signal",
      "net_names": ["FB", "COMP"],
      "recommendation": "Route away from noisy signals; use short, direct traces",
      "trace_width": null,
      "via_requirements": "Minimize vias in feedback path",
      "source_text": "Keep feedback traces short and away from the inductor and SW node.",
      "source_page": 16
    },
    {
      "net_type": "High Current Power",
      "net_names": ["VIN", "VOUT", "SW"],
      "recommendation": "Use wide traces or copper pours to minimize resistance and inductance",
      "trace_width": "Min 0.5mm (20mil) for currents up to 2A",
      "via_requirements": "Use multiple vias for current sharing",
      "source_text": "Power traces should be as wide as possible. Use copper pours for high-current paths.",
      "source_page": 15
    },
    {
      "net_type": "Thermal",
      "net_names": ["Thermal Pad", "PGND"],
      "recommendation": "Connect directly to ground plane with thermal vias",
      "trace_width": null,
      "via_requirements": "Array of 0.3mm vias, 0.6mm pitch, minimum 9 vias",
      "source_text": "Place an array of thermal vias under the exposed pad connecting to the ground plane.",
      "source_page": 17
    }
  ],
  "grounding_strategy": {
    "type": "Star Ground",
    "description": "Connect all ground returns to a single point under the IC",
    "analog_digital_separation": "Separate analog and power grounds; connect at star point",
    "ground_plane_requirement": "Continuous ground plane required under device and signal routing",
    "source_text": "Use a star ground topology with all returns connecting at a single point.",
    "source_page": 18
  },
  "thermal_management": {
    "copper_pour_requirement": "Maximize copper area connected to thermal pad for heat spreading",
    "via_pattern": "3x3 array of 0.3mm vias under thermal pad",
    "bottom_layer_connection": "Connect thermal vias to ground copper on bottom layer",
    "additional_notes": [
      "Do not place components directly under thermal pad on bottom layer",
      "Ensure via barrel is plated for thermal conductivity"
    ],
    "source_page": 17
  },
  "layer_stackup_notes": {
    "minimum_layers": 4,
    "recommended_stackup": "Signal-Ground-Power-Signal",
    "ground_plane_requirements": "Uninterrupted ground plane on layer 2 required",
    "notes": [
      "4-layer board recommended for best EMI performance",
      "2-layer board possible but requires careful ground plane design"
    ],
    "source_text": "A 4-layer PCB with internal ground and power planes is recommended.",
    "source_page": 18
  }
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL placement constraints are extracted
- [ ] ALL routing constraints are extracted
- [ ] Priorities are assigned based on datasheet emphasis
- [ ] Source text is quoted exactly
- [ ] Dimensions include units
- [ ] Source page numbers are 0-indexed and accurate
