**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior Hardware Systems Engineer.

**Objective:** Extract the "Golden Reference Design" from the datasheet, including component values, types, and connectivity rules, to generate a functional schematic netlist.

**Context:** The output will be used to automatically generate schematic symbols and component connections for the reference design circuit.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO application circuit or reference design exists, respond with: `{"error": "No reference design found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Locate All Application Circuits
Search for these sections:
- "Typical Application Circuit"
- "Application Schematic"
- "Reference Design"
- "Simplified Schematic"
- "Evaluation Board Schematic"

For EACH application circuit found, record:
- Circuit name/purpose
- Page number
- Whether it's a "typical" or "alternative" design

### Step 2: Extract ALL External Components
For EACH component in the reference design, extract:

| Field | Requirement |
|-------|-------------|
| `component_type` | Descriptive type (e.g., "Input Capacitor", "Feedback Resistor") |
| `designator` | Reference designator or prefix (e.g., "Cin", "R1", "Rfb") |
| `recommended_value` | Primary recommended value with unit |
| `min_value` | Minimum acceptable value (if specified) |
| `max_value` | Maximum acceptable value (if specified) |
| `tolerance` | Required tolerance (if specified) |
| `dielectric_type` | For capacitors: X7R, X5R, C0G, etc. |
| `voltage_rating` | Minimum voltage rating |
| `esr_requirement` | ESR constraints (if specified) |
| `package_size` | Recommended package (if specified) |
| `is_required` | `true` if required, `false` if optional |

### Step 3: Map Component Connectivity
For EACH component, describe how it connects:

| Field | Requirement |
|-------|-------------|
| `pin_1_connection` | What pin 1/positive connects to (IC pin name or net) |
| `pin_2_connection` | What pin 2/negative connects to |
| `placement_notes` | Physical placement requirements |

### Step 4: Extract Design Constraints and Stability Notes
Look for critical application notes:
- Stability requirements (ESR ranges, capacitor types)
- Loop compensation requirements
- Thermal considerations
- Component derating guidelines

### Step 5: Extract Multiple Configurations (if present)
If the datasheet shows different configurations (different output voltages, modes, etc.), extract each as a separate reference design.

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List components in schematic order (input to output, top to bottom)
2. **Completeness:** Include ALL components shown, even decoupling capacitors
3. **Exactness:** Use EXACT component values from schematic (not approximations)
4. **Pin Names:** Use EXACT IC pin names from the document
5. **Units:** Always include units (uF, kohm, nH, etc.)

---

## IF DATA NOT FOUND

- If component value is not specified: Use `"value": "see datasheet calculation"` with formula if provided
- If tolerance is not specified: Use `"tolerance": "standard"` (assume 1% for resistors, 10% for capacitors)
- If a component is optional: Set `"is_required": false` and note the benefit
- If multiple values are acceptable: Use `"recommended_value"` for the typical and `"alternate_values"` array for others

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [8, 12],
  "reference_designs": [
    {
      "design_name": "Typical 3.3V LDO Application",
      "design_type": "typical",
      "source_page": 12,
      "output_voltage": "3.3V",
      "output_current": "150mA max",
      "required_components": [
        {
          "component_type": "Input Capacitor",
          "designator": "Cin",
          "recommended_value": "1.0uF",
          "min_value": "0.7uF",
          "max_value": null,
          "tolerance": null,
          "dielectric_type": "X7R or X5R ceramic",
          "voltage_rating": "6.3V minimum",
          "esr_requirement": "Low ESR (<100mOhm)",
          "package_size": "0402 or larger",
          "is_required": true,
          "connectivity": {
            "pin_1_connection": "VIN",
            "pin_2_connection": "GND",
            "placement_notes": "Place within 2mm of VIN pin"
          }
        },
        {
          "component_type": "Output Capacitor",
          "designator": "Cout",
          "recommended_value": "2.2uF",
          "min_value": "1.0uF",
          "max_value": "22uF",
          "tolerance": null,
          "dielectric_type": "X7R or X5R ceramic",
          "voltage_rating": "6.3V minimum",
          "esr_requirement": "10mOhm to 500mOhm for stability",
          "package_size": "0603 or larger",
          "is_required": true,
          "connectivity": {
            "pin_1_connection": "VOUT",
            "pin_2_connection": "GND",
            "placement_notes": "Place as close to VOUT as possible"
          }
        },
        {
          "component_type": "Enable Pull-down Resistor",
          "designator": "Ren",
          "recommended_value": "10kohm",
          "min_value": "10kohm",
          "max_value": "100kohm",
          "tolerance": "1%",
          "dielectric_type": null,
          "voltage_rating": null,
          "esr_requirement": null,
          "package_size": null,
          "is_required": false,
          "connectivity": {
            "pin_1_connection": "EN",
            "pin_2_connection": "GND",
            "placement_notes": "Optional - keeps device disabled when EN floats"
          }
        },
        {
          "component_type": "Feedforward Capacitor",
          "designator": "Cff",
          "recommended_value": "10pF",
          "min_value": "5pF",
          "max_value": "22pF",
          "tolerance": null,
          "dielectric_type": "C0G/NP0",
          "voltage_rating": null,
          "esr_requirement": null,
          "package_size": null,
          "is_required": false,
          "connectivity": {
            "pin_1_connection": "NR",
            "pin_2_connection": "GND",
            "placement_notes": "Optional - improves transient response"
          }
        }
      ],
      "critical_notes": [
        "Output capacitor ESR is critical for loop stability",
        "Do not use tantalum capacitors if ESR > 500mOhm",
        "Input capacitor prevents input supply noise from affecting regulation"
      ]
    }
  ],
  "design_equations": [
    {
      "parameter": "Output Voltage (adjustable version)",
      "equation": "VOUT = VREF * (1 + R1/R2)",
      "notes": "VREF = 0.8V for adjustable version"
    }
  ],
  "layout_recommendations": [
    "Ground plane required under device",
    "Star ground topology recommended",
    "Keep high-current paths short"
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL components from the reference schematic are included
- [ ] Component values include units
- [ ] Pin connections use exact pin names from datasheet
- [ ] Required vs optional status is correct
- [ ] ESR and voltage rating requirements are captured
- [ ] Source page numbers are 0-indexed and accurate
