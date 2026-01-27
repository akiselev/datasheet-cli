**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Power Management Systems Engineer.

**Objective:** Extract the Power-Up/Power-Down sequencing requirements and group power pins by voltage domain for decoupling synthesis.

**Context:** The output will be used to design power supply sequencing circuits and specify decoupling capacitors for a PCB design.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO power supply information exists, respond with: `{"error": "No power supply information found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Identify ALL Power Rails
Search for: "Power Supply", "Power Pins", "Voltage Domains", "Power Distribution"

For EACH distinct power rail, extract:

| Field | Requirement |
|-------|-------------|
| `rail_name` | EXACT name (e.g., "VDD_CORE", "VDDIO", "AVDD") |
| `voltage_level` | Nominal voltage with tolerance if specified |
| `pins` | Array of ALL pin numbers/names for this rail |
| `current_typical` | Typical current consumption (if provided) |
| `current_max` | Maximum current consumption (if provided) |
| `source_page` | 0-indexed page number |

### Step 2: Extract Decoupling Requirements
For EACH power rail, find capacitor recommendations:

| Field | Requirement |
|-------|-------------|
| `rail_name` | Rail this applies to |
| `capacitors` | Array of capacitor specifications |
| `placement_notes` | Placement guidance (e.g., "within 3mm of pin") |

Each capacitor entry should include:
- `count`: Number required (or "1 per pin")
- `value`: Capacitance value (e.g., "100nF", "10uF")
- `type`: Capacitor type (e.g., "Bulk", "Local Bypass", "Ceramic X7R")
- `voltage_rating`: Minimum voltage rating if specified
- `esr_requirement`: ESR requirements if specified

### Step 3: Extract Sequencing Requirements
Search for: "Power Supply Sequencing", "Power-On Sequence", "Initialization", "Power-Up"

For EACH sequencing rule, extract:

| Field | Requirement |
|-------|-------------|
| `order_step` | Numeric sequence position (1, 2, 3...) |
| `rail` | Rail name with voltage |
| `condition` | What must be achieved (e.g., "Must reach 90% of nominal") |
| `timing_delay` | Delay relative to previous step (e.g., "Min 1ms after Step 1") |
| `notes` | Additional requirements (monotonicity, slew rate, etc.) |

### Step 4: Extract Power-Down Requirements (if present)
Search for: "Power-Down Sequence", "Shutdown", "Power Off"

Document any power-down sequencing that differs from reverse power-up.

### Step 5: Extract Current Consumption Data
Search for: "Current Consumption", "Power Consumption", "Operating Current"

For EACH operating mode, extract:
- Mode name (e.g., "Active", "Sleep", "Deep Sleep")
- Current consumption (typical and max)
- Conditions (clock frequency, peripherals active, etc.)

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List power rails alphabetically by rail_name
2. **Ordering:** List sequencing rules by order_step (ascending)
3. **Completeness:** Include ALL power pins even if they share a rail name
4. **Exactness:** Use EXACT rail names from datasheet (preserve case)

---

## IF DATA NOT FOUND

- If no sequencing rules exist: Set `"sequencing_rules": []` with note `"No sequencing requirements specified"`
- If decoupling is not specified for a rail: Include rail with `"decoupling_capacitors": null`
- If current consumption is not specified: Omit from that rail's entry
- If power-down differs from reverse power-up: Document explicitly; otherwise omit

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [8, 15, 22],
  "power_rails": [
    {
      "rail_name": "VDD_CORE",
      "voltage_level": "1.2V +/-5%",
      "pins": ["A1", "A2", "B5", "C3"],
      "current_typical": "50mA",
      "current_max": "120mA",
      "decoupling_capacitors": [
        {
          "count": 1,
          "value": "10uF",
          "type": "Bulk ceramic",
          "voltage_rating": "6.3V minimum",
          "esr_requirement": null
        },
        {
          "count": "1 per pin",
          "value": "100nF",
          "type": "Local bypass ceramic X7R",
          "voltage_rating": null,
          "esr_requirement": "Low ESR"
        }
      ],
      "placement_notes": "Place 100nF capacitors within 2mm of each power pin",
      "source_page": 15
    },
    {
      "rail_name": "VDD_IO",
      "voltage_level": "3.3V or 1.8V (selectable)",
      "pins": ["D1", "D2", "E5"],
      "current_typical": "10mA",
      "current_max": "30mA",
      "decoupling_capacitors": [
        {
          "count": 1,
          "value": "4.7uF",
          "type": "Bulk",
          "voltage_rating": null,
          "esr_requirement": null
        }
      ],
      "placement_notes": null,
      "source_page": 15
    }
  ],
  "sequencing_rules": [
    {
      "order_step": 1,
      "rail": "VDD_CORE (1.2V)",
      "condition": "Must reach 90% of nominal value",
      "timing_delay": null,
      "notes": "Supply must rise monotonically"
    },
    {
      "order_step": 2,
      "rail": "VDD_IO (3.3V)",
      "condition": "May power on after VDD_CORE stable",
      "timing_delay": "Minimum 1ms after Step 1",
      "notes": null
    },
    {
      "order_step": 3,
      "rail": "VDD_PLL (1.2V)",
      "condition": "Power on after VDD_CORE and VDD_IO",
      "timing_delay": "Minimum 100us after Step 2",
      "notes": "Do not apply before VDD_IO"
    }
  ],
  "power_down_rules": [
    {
      "order_step": 1,
      "rail": "VDD_PLL",
      "notes": "Disable PLL first"
    }
  ],
  "current_consumption": [
    {
      "mode": "Active",
      "conditions": "All peripherals active, 80MHz clock",
      "current_typical": "45mA",
      "current_max": "65mA"
    },
    {
      "mode": "Sleep",
      "conditions": "CPU halted, peripherals active",
      "current_typical": "8mA",
      "current_max": "12mA"
    },
    {
      "mode": "Deep Sleep",
      "conditions": "RTC only",
      "current_typical": "2uA",
      "current_max": "5uA"
    }
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL power rails are identified with ALL their pins
- [ ] Decoupling requirements include count, value, and type
- [ ] Sequencing rules are in correct numeric order
- [ ] Current consumption data includes operating conditions
- [ ] Source page numbers are accurate (0-indexed)
